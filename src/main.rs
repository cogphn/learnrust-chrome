extern crate csv;
extern crate argparse;

use sqlite::State;
use std::error::Error;
use csv::Writer;
//use serde::Serialize;
//use std::fs::File;
//use std::io::prelude::*;
use chrono::{DateTime, NaiveDateTime, Utc, TimeZone, Duration};
use argparse::{ArgumentParser, Store};

#[path = "config_structs.rs"]
mod config_structs;

/*
#[derive(Serialize)]
struct DownloadsDataRow<'a> {
    id: &'a str,
    guid: &'a str,
    current_path: &'a str,
    target_path: &'a str,
    start_time: &'a str,
    start_time_dtutc: &'a str,
    received_bytes: &'a str,
    total_bytes: &'a str,
    state: &'a str,
    danger_type: &'a str,
    interrupt_reason: &'a str,
    hash: &'a str,
    end_time: &'a str,
    end_time_dtutc: &'a str,
    opened: &'a str,
    last_access_time: &'a str,
    last_access_time_dtutc: &'a str,
    transient: &'a str,
    referrer: &'a str,
    site_url: &'a str,
    tab_url: &'a str,
    tab_referrer_url: &'a str,
    http_method: &'a str,
    by_ext_id: &'a str,
    by_ext_name: &'a str,
    etag: &'a str,
    last_modified: &'a str,
    last_modified_dtutc: &'a str,
    mime_type: &'a str,
    original_mime_type:&'a str,
}
*/

fn get_timestamp(gts: i64) -> String {
    let _c_starttime = Utc.ymd(1601, 1, 1).and_hms_milli(0, 0, 0, 0);
    let _d = Duration::microseconds(gts);
    return  (_c_starttime + _d).to_string() ;
}

fn get_moz_ts(mts: i64) -> String {
    //TODO: fix for higher res timestamps
    let mts_1 = mts / 1000000;
    let d = DateTime::<Utc>::from_utc(NaiveDateTime::from_timestamp(mts_1,0), Utc);
    return d.to_string();
}


fn get_table_data(query: &str, infile: &str, output_path: &str, fields: Vec<config_structs::Field>) -> i32 {
    let connection = sqlite::open(infile).unwrap();
    let mut statement = connection.prepare(query).unwrap();
    
    println!("[*] Reading URL data...");
    let mut rowtrack =0;
    
    let mut wtr = match Writer::from_path(output_path){
        Ok(w) => w,
        Err(e) => panic!("Cannot open file for writing URL output file for writing. Error: {}",e)
    };
    
    //write header
    let mut colnames = Vec::new();
    for f in &fields {
        let fname = f.name.to_string();
        let mut fname1 = f.name.to_owned();
        colnames.push(fname);
        if f.coltype == "chrome_ts"{
            fname1 += "_dtutc";
            colnames.push(fname1);
        } else if f.coltype == "moz_ts" {
            fname1 += "_dtutc";
            colnames.push(fname1);
        }
    }
    match wtr.write_record(colnames) {
        Ok(x) => x,
        Err(e) => println!("[!] error writing URL data: {}",e)
    };
    
    while let State::Row = statement.next().unwrap() {
        let mut rowdata = vec![];

        for f in &fields {
            let idx = f.ord;
            let mut colval_is_null = false;
            if f.nullable == 1 {
                let d = match statement.read::<String>(idx) {
                    Ok(x) => x,
                    Err(_e) => "NULL".to_string()
                };
                if d == "NULL" {
                    colval_is_null= true;
                }
                rowdata.push(d);
            } else {
                let d = statement.read::<String>(idx).unwrap();
                rowdata.push(d);
            }
            

            if f.coltype == "chrome_ts"{
                let tsval :i64 = statement.read::<i64>(idx).unwrap();
                let dtutc = get_timestamp(tsval);
                let str_dtutc = &dtutc.replace(" UTC","");
                rowdata.push(str_dtutc.to_string());
            }

            if f.coltype == "moz_ts" {
                let mut tsval: i64 = 0;
                if colval_is_null == false {
                    tsval = statement.read::<i64>(idx).unwrap();
                }
                let dtutc = get_moz_ts(tsval);
                let str_dtutc = &dtutc.replace(" UTC","");
                rowdata.push(str_dtutc.to_string());
            }
        }
        
        match wtr.serialize(rowdata){
            Ok(x) => x,
            Err(e) => println!("Error: {}",e)
        };
        rowtrack +=1;
    } //while there's a row to read 
        
    println!("[*] Writing data...");
    match wtr.flush() {
        Ok(()) => println!("[*] URL data: wrote {} rows", rowtrack),
        Err(e) => println!("[!] Error writing URL data: {}",e)
    };

    return 0;
}


fn main() -> Result<(), Box<dyn Error>> {
    println!("[*] Starting...");

    let mut infile = "".to_string();
    let mut output_path = "output.csv".to_string(); 
    let mut tablename = "".to_string();
    let mut configpath = "".to_string();
    {
        let mut ap = ArgumentParser::new();
        ap.set_description("Browser Hisory parse");
        ap.refer(&mut output_path)
            .add_option(&["-o","--output"], 
            Store,
            "output file name"
        );

        ap.refer(&mut infile)
        .add_option(&["-i","--input"], 
            Store,
            "full path to History file"
        ).required();

        ap.refer(&mut tablename).add_option(
            &["-t","--table"], 
            Store, 
            "table name (supported: urls, downloads,download_url_chains)"
        ).required();

        ap.refer(&mut configpath).add_option(
            &["-c","--config"],
            Store,
            "table config file"
        ).required();
        
        ap.parse_args_or_exit();
    }

    let mut useconfig :bool = false;

    if configpath != ""{
        //config file specified
        println!("[*] Config file: {}", configpath);
        let configjsonstring = match std::fs::read_to_string(configpath) { 
            Ok(x) => x.to_string(),
            Err(e) => panic!("[!] cannot read config file: {}",e)
        };
        let configdata : config_structs::Root = serde_json::from_str(&configjsonstring).expect("error loading config data");

        for t in &configdata.tables {
            if t.name == tablename {
                useconfig = true;
                let query = &t.config.query;
                let fields = &t.config.fields;
                let _ret = get_table_data(&query, &infile, &output_path, fields.to_vec());
                break;
            }
        }
    }

    if useconfig {
        println!("[.] Done.");
        std::process::exit(0);
    }

    println!("[!] tablename not found in config file");
    
    Ok(())
}


