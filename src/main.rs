use std::collections::HashMap;
use chrono::DateTime;
use chrono::Duration;
use chrono::Utc;
use chrono_tz::Tz;
use chrono_tz::OffsetComponents;
use tokio::task;
use lambda_http::{run, service_fn, Error, IntoResponse, Request, Response};
use aws_sdk_dynamodb::{model::AttributeValue};


struct DstChange {
    timezone_name: String, 
    next_dst_change: DateTime<Tz>
}

struct DstError {
    timezone_name: String, 
    reason: String
}

async fn function_handler(_event: Request) -> Result<impl IntoResponse, Error> {
    // Extract some useful information from the request
    dst_calculation().await?;
    // Return something that implements IntoResponse.
    // It will be serialized to the right response event automatically by the runtime
    let resp = Response::builder()
        .status(200)
        .header("content-type", "text/html")
        .body("Hello AWS Lambda HTTP request")
        .map_err(Box::new)?;
    Ok(resp)
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        // disabling time is handy because CloudWatch will add the ingestion time.
        .without_time()
        .init();

    run(service_fn(function_handler)).await
}


async fn dst_calculation() -> Result< (), String> {
    use std::time::Instant;
    let now = Instant::now();

    let list_tz = [
        "Australia/Adelaide",
        "toto en vacances",
        "Africa/Abidjan",
        "Asia/Calcutta",
        "bernard a la plage",
        "America/Godthab",
        "Europe/Paris", 
        "Europe/Amsterdam",
        "Europe/Andorra",
        "Europe/Athens",
        "Europe/Belfast",
        "Europe/Belgrade",
        "Europe/Berlin",
        "Europe/Bratislava",
        "Europe/Brussels",
        "Europe/Bucharest",
        "Europe/Budapest",
        "Europe/Busingen",
        "Europe/Chisinau",
        "Europe/Copenhagen",
        "Europe/Dublin",
        "Europe/Gibraltar",
    ];

    let future_vec : Vec<_> = list_tz
        .iter()
        .map(|time_zone| task::spawn(get_dst_change(time_zone.to_string())))
        .collect();

    let responses = futures::future::join_all(future_vec).await;

    let mut dst_to_save : Vec<Vec<DstChange>> = Vec::new();
    
    for response in responses {
        match response {
            Ok(result) => match result {
                Ok(dst) => {
                    dst_to_save.push(dst);
                },
                Err(error) => println!("Error getting next dst change for tz {}, for reason: {}", error.timezone_name, error.reason),
            },
            Err(err) => println!("Error executing task {:?}", err),
        };
    }

    // batch write to dyndb
    save_to_db(dst_to_save).await.expect("failed to save into db");

    let elapsed = now.elapsed();
    println!("Elapsed: {:.2?}", elapsed);

    Ok(())

}

/// save to db takes a dst change list and save it to dyndb
/// this is only doing the save
/// later improvment can be to make a batch
async fn save_to_db(dst_to_save : Vec<Vec<DstChange>>) -> Result<String, String> {
    let shared_config = aws_config::from_env().load().await;
    let dyn_client = aws_sdk_dynamodb::Client::new(&shared_config);
    
    for dst in dst_to_save {
        let mut maps: HashMap<String, AttributeValue> = HashMap::new();
        maps.insert("timezone_name".to_string(), AttributeValue::S(dst[0].timezone_name.clone()));
        maps.insert("next_dst_change".to_string(), AttributeValue::S(dst[0].next_dst_change.to_string()));
        maps.insert("second_dst_change".to_string(), AttributeValue::S(dst[1].next_dst_change.to_string()));
        
        // todo: set the proper table name
        let result = dyn_client
            .put_item()
            .table_name("dst_change")
            .set_item(Some(maps))
            .send()
            .await;

        match result {
            Ok(message) => println!("tz inserted with success {}, with details {:?}", dst[0].timezone_name, message),
            Err(error) => println!("error inserting tz {} with details: {:?}", dst[0].timezone_name, error),
        }
    }

    Ok(String::from("success"))
}

/// get next dst change take a time zone name and return the next dst based
/// on date time now an improvment can be to make it based on a passed date
/// so you can use the same script for the next two
async fn get_dst_change(time_zone_name: String) -> Result<Vec<DstChange>,DstError> {
    let tz = time_zone_name.parse();

    let tz: Tz = match tz {
        Ok(timezone) => timezone,
        Err(err) => return Err(DstError{
            timezone_name: time_zone_name,
            reason: String::from(err),
        }),  
    };

    let current_utc_date = Utc::now();
    let current_local_date = current_utc_date.with_timezone(&tz);
    
    let mut dst_changes: Vec<DstChange> = Vec::new();
    // find next one
    match get_next_dst_change(current_local_date){
        Ok(dst_change) => {
            match get_next_dst_change(dst_change.next_dst_change){
                Ok(dst_change2) => dst_changes.push(dst_change2),
                Err(err) => println!("error with dst: {}",err.timezone_name)
            };
            dst_changes.push(dst_change);
        },
        Err(error) => {
            println!("error with the timezone: {} for the following reason {}", error.timezone_name, error.reason);
            return Err(error);
        }
    };

    Ok(dst_changes)

}

fn get_next_dst_change(date_time: DateTime<Tz>) -> Result<DstChange, DstError> {
    let mut dst_time = date_time.clone();
    
    let one_day = Duration::days(1);
    let one_quarter = Duration::days(90);
    let one_month = Duration::days(30);
    let one_hour = Duration::hours(1);
    let one_minute = Duration::minutes(1);

    let mut counter : i16 = 0;

    // find next quarter when it changes
    while date_time.offset().dst_offset() == dst_time.offset().dst_offset() {
        dst_time = dst_time + one_quarter;
        counter = counter + 1;
        
        if counter == 4 {
            return Err(DstError{
                timezone_name: date_time.timezone().to_string(),
                reason: String::from("no dst change")
            });
        }
    }

    // find the month
    while date_time.offset().dst_offset() != dst_time.offset().dst_offset() {
        dst_time = dst_time - one_month;
    }

    // find the day
    while date_time.offset().dst_offset() == dst_time.offset().dst_offset() {
        dst_time = dst_time + one_day;
    }
    // find the hour
    while date_time.offset().dst_offset() != dst_time.offset().dst_offset() {
        dst_time = dst_time - one_hour;
    }

    // find the minute
    while date_time.offset().dst_offset() == dst_time.offset().dst_offset() {
        dst_time = dst_time + one_minute;
    }

    // remove one minute to get the minute before it changes
    dst_time = dst_time - one_minute;

    Ok(DstChange {
        timezone_name : date_time.timezone().to_string(),
        next_dst_change : dst_time
    })
}
