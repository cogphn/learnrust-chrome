extern crate csv;
extern crate argparse;

use sqlite::State;
use std::error::Error;
use csv::Writer;
use serde::Serialize;
//use std::fs::File;
//use std::io::prelude::*;
use chrono::{DateTime, NaiveDateTime, Utc, TimeZone, Duration};
use argparse::{ArgumentParser, Store};

#[path = "config_structs.rs"]
mod config_structs;


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
            "optional table config file"
        );
        
        ap.parse_args_or_exit();
    }

    let _urlop = format!("{output_path}");
    let _dlop = format!("{output_path}");
    let _dlucop = format!("{output_path}");
    
    let mut useconfig :bool = false;

    if configpath != ""{
        //config file specified
        println!("Config file specified: {}", configpath);
        let configjsonstring = match std::fs::read_to_string(configpath) { 
            Ok(x) => x.to_string(),
            Err(e) => panic!("cannot read config file: {}",e)
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
            //configindex +=1;
        }
    }

    if useconfig {
        println!("[.] Done. (config used)");
        std::process::exit(0);
    }

    

    //assuming sqlite
    let connection = sqlite::open(infile).unwrap();

    if tablename == "urls"{
            let mut urls_statement = connection.prepare("select id, url, title, visit_count, typed_count, last_visit_time, hidden from urls").unwrap();
            println!("[*] Reading URL data...");
            let mut rowtrack =0;
            let mut u_cwtr = match Writer::from_path(_urlop){
                Ok(w) => w,
                Err(e) => panic!("Cannot open file for writing URL output file for writing. Error: {}",e)
            };
            match u_cwtr.write_record(&["id","url","title","visit_count","typed_content","last_visit_time","last_visit_time_dtutc","hidden"]) {
                Ok(x) => x,
                Err(e) => println!("[!] error writing URL data: {}",e)
            };
            while let State::Row = urls_statement.next().unwrap() {
                let lv = urls_statement.read::<i64>(5).unwrap();
                let lv_ts_utc = get_timestamp(lv).replace(" UTC","");
                
                match u_cwtr.write_record(
                    &[
                        &urls_statement.read::<String>(0).unwrap(),
                        &urls_statement.read::<String>(1).unwrap(),
                        &urls_statement.read::<String>(2).unwrap(),
                        &urls_statement.read::<String>(3).unwrap(),
                        &urls_statement.read::<String>(4).unwrap(),
                        &urls_statement.read::<String>(5).unwrap(),
                        &lv_ts_utc,
                        &urls_statement.read::<String>(6).unwrap()
                    ]
                ){
                    Ok(x) => x,
                    Err(e) => println!("[!] error at row {}:{}",rowtrack, e)
                };
                rowtrack +=1;
            }
            println!("[*] Writing URL Data...");
            match u_cwtr.flush() {
                Ok(()) => println!("[*] URL data: wrote {} rows", rowtrack),
                Err(e) => println!("[!] Error writing URL data: {}",e)
            };
        }  else if tablename == "moz_historyvisits" {
            println!("[*] reading {} data...", tablename);
            let statement = "select a.id as hitoryvisitid, a.from_visit, a.place_id, 
                a.visit_type, a.session, b.url, b.title, b.rev_host, b.visit_count,
                b.hidden, b.typed, b.frecency, b.last_visit_date, b.guid, b.foreign_count, b.url_hash, b.description, b.preview_image_url,
                b.origin_id, b.site_name
                from moz_historyvisits a join moz_places b on a.place_id = b.id ";
            let mut results = connection.prepare(statement).unwrap();
            let mut cwtr = match Writer::from_path(output_path){
                Ok(w) => w,
                Err(e) => panic!("cannot open output file for writing: {}",e)
            };

            let mut rowtrack = 0;
            match cwtr.write_record(&[
                "hitoryvisitid","from_visit","place_id","visit_type",
                "session","url","title","rev_host","visit_count","hidden",
                "typed","frecency","last_visit_date","last_visit_date_dtutc","guid","foreign_count",
                "url_hash","description","preview_image_url","origin_id","site_name"
                ]){
                Ok(x) => x,
                Err(e) => println!("[!] error writing header for segment data: {}",e)
            }

            while let State::Row = results.next().unwrap() {
                let title = match results.read::<String>(6) {
                    Ok(h) => h,
                    Err(_e) => "NULL".to_string()
                };
                let description = match results.read::<String>(16) {
                    Ok(h) => h,
                    Err(_e) => "NULL".to_string()
                };
                let preview_image_url = match results.read::<String>(17) {
                    Ok(x) => x,
                    Err(_e) => "NULL".to_string()
                };
                let site_name = match results.read::<String>(19) {
                    Ok(x) => x,
                    Err(_e) => "NULL".to_string()
                };
                let url_hash = match results.read::<String>(15) {
                    Ok(h) => h,
                    Err(_e) => "NULL".to_string()
                };
                let i64_lvd : i64 = match results.read::<i64>(12) {
                    Ok(x) => x,
                    Err(_e) => 0
                };
                //let lvd = results.read::<i64>(8).unwrap();
                let last_visit_date_dtutc = get_moz_ts(i64_lvd).replace(" UTC","");
                match cwtr.write_record(
                    &[
                        &results.read::<String>(0).unwrap(),
                        &results.read::<String>(1).unwrap(),
                        &results.read::<String>(2).unwrap(),
                        &results.read::<String>(3).unwrap(),
                        &results.read::<String>(4).unwrap(),
                        &results.read::<String>(5).unwrap(),
                        &title,
                        &results.read::<String>(7).unwrap(),
                        &results.read::<String>(8).unwrap(),
                        &results.read::<String>(9).unwrap(),
                        &results.read::<String>(10).unwrap(),
                        &results.read::<String>(11).unwrap(),
                        &results.read::<String>(12).unwrap(),
                        &last_visit_date_dtutc,
                        &results.read::<String>(13).unwrap(),
                        &results.read::<String>(14).unwrap(),
                        &url_hash,
                        &description,
                        &preview_image_url,
                        &results.read::<String>(18).unwrap(),
                        &site_name
                    ]
                ){
                    Ok(x) => x,
                    Err(e) => println!("[!] Error at row {}:{}",rowtrack,e)
                };
                rowtrack+=1;
            }
            match cwtr.flush(){
                Ok(()) => println!("[*] {}: wrote {} rows", tablename, rowtrack),
                Err(e)  => println!("[!] error writing data for {}: {}",tablename, e)
            };

        }
    println!("[.] Done.");
    Ok(())
}


