extern crate csv;

use sqlite::State;
use std::error::Error;
use csv::Writer;
use serde::Serialize;
use std::fs::File;
use std::io::prelude::*;

use chrono::Utc;
use chrono::TimeZone;
use chrono::Duration;

extern crate argparse;

use argparse::{ArgumentParser, Store};


#[derive(Serialize)]
struct UrlDataRow<'a> {
    id: &'a str,
    url: &'a str,
    title: &'a str,
    visit_count: &'a str,
    typed_content: &'a str,
    last_visit_time: &'a str,
    last_visit_dtutc: &'a str,
    hidden: &'a str,
}
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


fn main() -> Result<(), Box<dyn Error>> {
    println!("[*] Starting...");

    let mut infile = "".to_string();
    let mut output_base_path = "chrome_parse_output_".to_string(); 
    {
        let mut ap = ArgumentParser::new();
        ap.set_description("Chrome Hisory parse");
        ap.refer(&mut output_base_path)
            .add_option(&["-o","--output"], Store,
            "output base path"
        );

        ap.refer(&mut infile)
        .add_option(&["-i","--input"], Store,
            "full path to History file"
        ).required();
        
        ap.parse_args_or_exit();
    }
    let _urls_file_suffix = "chrome_urls.csv".to_string();
    let _downloads_file_suffix = "chrome_downloads.csv".to_string();
    let _dl_url_chains_suffix = "chrome_downloads_url_chains.csv".to_string();
    
    
    let _urlop = format!("{output_base_path}{_urls_file_suffix}");
    let _dlop = format!("{output_base_path}{_downloads_file_suffix}");
    let _dlucop = format!("{output_base_path}{_dl_url_chains_suffix}");
    
    let connection = sqlite::open(infile).unwrap();
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
    
    
    // Downloads

    let mut downloads_statement = connection.prepare("select id,guid,current_path,target_path,start_time,received_bytes,total_bytes,state,danger_type,interrupt_reason,hash,end_time,opened,last_access_time,transient,referrer,site_url,tab_url,tab_referrer_url,http_method,by_ext_id,by_ext_name,etag,last_modified,mime_type,original_mime_type from downloads").unwrap();
    let mut dl_rowtrack =0;
    let mut wtr = Writer::from_writer(vec! []);
    println!("[*] Reading downloads data...");
    while let State::Row = downloads_statement.next().unwrap() {

        let start_time = downloads_statement.read::<i64>(4).unwrap();
        let start_time_dtutc = get_timestamp(start_time).replace(" UTC","");
        
        let end_time = downloads_statement.read::<i64>(11).unwrap();
        let end_time_dtutc = get_timestamp(end_time).replace(" UTC","");
        
        let last_access_time = downloads_statement.read::<i64>(13).unwrap();
        let last_access_time_dtutc = get_timestamp(last_access_time).replace(" UTC", "");

        let last_modified = downloads_statement.read::<i64>(23).unwrap();
        let last_modified_dtutc = get_timestamp(last_modified).replace(" UTC","");

        let _ser_result = match wtr.serialize(DownloadsDataRow {
            id: &downloads_statement.read::<String>(0).unwrap(),
            guid: &downloads_statement.read::<String>(1).unwrap(),
            current_path: &downloads_statement.read::<String>(2).unwrap(),
            target_path: &downloads_statement.read::<String>(3).unwrap(),
            start_time: &downloads_statement.read::<String>(4).unwrap(),
            start_time_dtutc: &start_time_dtutc,
            received_bytes: &downloads_statement.read::<String>(5).unwrap(),
            total_bytes: &downloads_statement.read::<String>(6).unwrap(),
            state: &downloads_statement.read::<String>(7).unwrap(),
            danger_type: &downloads_statement.read::<String>(8).unwrap(),
            interrupt_reason: &downloads_statement.read::<String>(9).unwrap(),
            hash: &downloads_statement.read::<String>(10).unwrap(),
            end_time: &downloads_statement.read::<String>(11).unwrap(),
            end_time_dtutc: &end_time_dtutc,
            opened: &downloads_statement.read::<String>(12).unwrap(),
            last_access_time: &downloads_statement.read::<String>(13).unwrap(),
            last_access_time_dtutc: &last_access_time_dtutc,
            transient: &downloads_statement.read::<String>(14).unwrap(),
            referrer: &downloads_statement.read::<String>(15).unwrap(),
            site_url: &downloads_statement.read::<String>(16).unwrap(),
            tab_url: &downloads_statement.read::<String>(17).unwrap(),
            tab_referrer_url: &downloads_statement.read::<String>(18).unwrap(),
            http_method: &downloads_statement.read::<String>(19).unwrap(),
            by_ext_id: &downloads_statement.read::<String>(20).unwrap(),
            by_ext_name: &downloads_statement.read::<String>(21).unwrap(),
            etag: &downloads_statement.read::<String>(22).unwrap(),
            last_modified: &downloads_statement.read::<String>(23).unwrap(),
            last_modified_dtutc: &last_modified_dtutc,
            mime_type: &downloads_statement.read::<String>(24).unwrap(),
            original_mime_type: &downloads_statement.read::<String>(25).unwrap()
        }) {
            Ok(r) => r,
            Err(e) => println!("[warn] Error seralizing row {}: {}",dl_rowtrack,e)
        };
        dl_rowtrack += 1;
    }

    let dl_data = wtr.into_inner();
    println!("[*] writing downlods data...");
    let mut dl_output = File::create(_dlop)?; 
    let dl_data_write_result = match dl_output.write_all(&dl_data.unwrap()) {
        Ok(i) => i,
        Err(e) => panic!("Error occured: {}",e)
    };
    
    if dl_data_write_result != () {
        println!("[*] Downloads Data: error: {:?}",dl_data_write_result);
    }else{
        println!("[*] Downloads data: wrote {} rows", dl_rowtrack);
    }


    ////////////////////////////////////////////// 
    //
    let mut dl_url_chains = connection.prepare("select id, chain_index, url from downloads_url_chains").unwrap();
    let mut cwtr = match Writer::from_path(_dlucop){
        Ok(w) => w,
        Err(e) => panic!("Cannot open dl url chains for writing: {}",e)
    };

    let mut dl_url_chain_rowtrack = 0;
    match cwtr.write_record(&["id","chain_index","url"]){
        Ok(x) => x,
        Err(e) => println!("[!] error writing data for download_url_chains: {}",e)
    }
    while let State::Row = dl_url_chains.next().unwrap() {
        match cwtr.write_record(
            &[
                &dl_url_chains.read::<String>(0).unwrap(),
                &dl_url_chains.read::<String>(1).unwrap(),
                &dl_url_chains.read::<String>(2).unwrap()]
        ){
            Ok(x) => x,
            Err(e) => println!("[!] Error at row {}:{}",dl_url_chain_rowtrack,e)
        };
        dl_url_chain_rowtrack+=1;
    }
    match cwtr.flush(){
        Ok(()) => println!("[*] wrote {} rows", dl_url_chain_rowtrack),
        Err(e)  => println!("[!] error writing data for download_url_chains: {}",e)
    };

    println!("[.] Done.");
    Ok(())
}


