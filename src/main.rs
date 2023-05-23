use aws_lambda_events::event::cloudwatch_events::CloudWatchEvent;
use lambda_runtime::{run, service_fn, Error, LambdaEvent};
use aws_config::meta::region::RegionProviderChain;
use aws_sdk_rds as rds;
use aws_sdk_secretsmanager as secretsmanager;
use aws_config;
use tokio_postgres as PgClient;
use rustls;
use tokio_postgres_rustls;
use tokio::time::{sleep, Duration};
use serde_json as json;
use serde::Deserialize;
use std::io::BufReader;
use std::collections::HashMap;
use rand::{Rng, thread_rng};
use rand::distributions::Alphanumeric;

#[derive(Debug, Default, Clone)]
struct RdsEndpoint {
    endpoint: String,
    port: i32,
    db_identifier: String,
    app_tag: String,
}

#[derive(Debug, Default, Clone, Deserialize)]
struct RdsCreds {
    username: String,
    password: String,
}

#[derive(Debug, Default, Clone, Deserialize)]
struct RdsPasswords {
    application: String,
    migration: String,
    dbowner: String,
}


// Handler
//
async fn function_handler(event: LambdaEvent<CloudWatchEvent>) -> Result<(), Error> {
    // Start
    println!("::Starting handler... I will print the payload ... \n\n");
    //println!("  Received event: {:?} \n\n", event);

    // Aws credentials
    let region_provider = RegionProviderChain::default_provider().or_else("us-east-1");
    let shared_config = aws_config::from_env().region(region_provider).load().await;

    // Aws clients
    let rds_client = rds::Client::new(&shared_config);
    let sm_client = secretsmanager::Client::new(&shared_config);

    // Event payload
    let payload: CloudWatchEvent = event.payload;
    //println!("  Print payload:  {:?} \n\n", payload);

    // Get db instance ARN from Event
    let event_map = json::from_value::<HashMap<String, json::Value>>(payload.detail.into())?;
    let arn_ref = &event_map["responseElements"]["dBInstanceArn"].as_str().unwrap();
    let arn = (*arn_ref).to_string();
    println!("  DbInstance ARN: {:?} ", arn);

    if arn.len() >= 5 {
        println!("  Handling: {:?}", arn);
        // Endpoint
        let re: RdsEndpoint = get_endpoint(&rds_client, &arn).await?;

        // Secret Manager name
        let sm_name = format!("{}-{}-rds-conn-string", re.app_tag, re.db_identifier);
        println!("  {}", sm_name);

        // Call secret manager and get the secret
        let rds_creds = get_secret(&sm_client, &sm_name).await?;
        //println!("{:?}", rds_creds);

        // Generate random passwords
        let n_rds_pass: RdsPasswords = (generate_random_pass().await).unwrap();

        // Call postgress
        let _pg = pg(re.clone(), rds_creds, &n_rds_pass).await?;

        // Save screts to sm
        save_secrets(&n_rds_pass, re, &sm_client).await?;
    }
    println!("::Ending ...");
    Ok(())
}


// Get RDS endpoint
//
async fn get_endpoint(client: &rds::Client, arn: &String) -> Result<RdsEndpoint, Error> {
    println!(":::Starting get_endpoint ...");
    // Aws client
    let mut result = client.describe_db_instances().db_instance_identifier(arn).send().await?;

    // Initialization
    let mut r: RdsEndpoint = Default::default();
    let mut db: &rds::types::DbInstance;
    // Tags structures
    let t: rds::types::builders::TagBuilder = rds::types::Tag::builder();
    let tags: Vec<rds::types::Tag>;
    let tg: rds::types::Tag = t.key("App").build();
    let tg_key: String = tg.key.unwrap();

    // Can I desconstruct db_instances in Some?
    if let Some(i) = result.db_instances() {
        db = &i[0];       // we should get just one result as using aws arn
   
        // CreateDbInstance event, db is not ready yet. I feel happy producing ugly code
        while let None = db.endpoint() {
            sleep(Duration::from_millis(300000)).await;
            result = client.describe_db_instances().db_instance_identifier(arn).send().await?;
            db = &result.db_instances().unwrap()[0];
        }

        // Filling RdsEndpoint struct
        r.endpoint = db.endpoint().unwrap().address().unwrap().to_string();
        r.port = db.endpoint().unwrap().port();
        r.db_identifier = db.db_instance_identifier().unwrap().to_string();
        
        // Looping on tags to get app_tag
        tags = db.tag_list.clone().unwrap();
        for j in tags {
            println!("   Tags loop...  {:?}  ", j);
            //println!("{:?}", j);
            if j.key.unwrap() == tg_key {
                println!("   tag App matched");
                r.app_tag = j.value.unwrap_or_default();
                println!("   app_tag is {:?}", r.app_tag);
            }
        }
        println!(":::Finalizing get_endpoint: {:?}", r);
    }
    Ok(r)
}


async fn get_secret(client: &secretsmanager::Client, name: &str) -> Result<RdsCreds, Error> {
    println!(":::Starting get_secret ...");
    // Aws client
    let resp = client.get_secret_value().secret_id(name).send().await?;
    let secret = resp.secret_string().unwrap();
    //println!("Value: {}", resp.secret_string().unwrap_or("No value!"));
    
    // Secret handler 
    let to_trim_s: &[char] = &['"'];
    let creds: RdsCreds = json::from_str(secret)?;
    let user = creds.username.trim_matches(to_trim_s);
    let pass = creds.password.trim_matches(to_trim_s);
    println!("   Finalizing get_secret");
    Ok( RdsCreds {username: user.to_owned(), password: pass.to_owned()} )
}


async fn pg(db: RdsEndpoint, pw: RdsCreds, npw: &RdsPasswords) -> Result<(), Error> {
    println!(":::Starting pg ...");
    // Creating cert store
    let custom_cert = include_bytes!("global-bundle.pem");
    let mut reader = BufReader::new(&custom_cert[..]);
    let mut cert_store = rustls::RootCertStore::empty();
    cert_store.add_parsable_certificates(&rustls_pemfile::certs(&mut reader).unwrap());

    // Include function
    let pg_function = include_str!("function.in");

    // Rustls client config
    let config = rustls::ClientConfig::builder()
        .with_safe_defaults()
        .with_root_certificates(cert_store)
        .with_no_client_auth();
    let tls = tokio_postgres_rustls::MakeRustlsConnect::new(config);

    // Make connection
    let conn = format!("dbname={} host={} port={} user={} password={} sslmode={}", "postgres", db.endpoint, db.port, pw.username, pw.password, "prefer");
    let (client, connection) = PgClient::connect(&conn, tls).await?;

    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("   connection error: {}", e);
        }
    });
    
    // Use conneciton
    println!("   connecting ... ");
    let rows = client.query_one("SELECT 1", &[]).await?;
    println!("   check conn  {:?}", rows);
    let f_batch = client.batch_execute(pg_function).await?;
    println!("  create role_config function result:  {:?}", f_batch);
    let rows2 = client.query("SELECT role_config(true,true,null,null,null, $1, $2, $3)", &[&npw.application, &npw.migration, &npw.dbowner]).await?;
    println!("  execute role_config result  {:?}", rows2);
    println!(":::Finalizing pg");
    Ok(())
}

async fn generate_random_pass() -> Result<RdsPasswords, ()> {
    let mut rng = thread_rng();
    let app = (0..13).map(|_| rng.sample(Alphanumeric) as char).collect();
    let mig = (0..13).map(|_| rng.sample(Alphanumeric) as char).collect();
    let dbo = (0..13).map(|_| rng.sample(Alphanumeric) as char).collect();

    Ok(RdsPasswords { application: app, migration: mig, dbowner: dbo })
}

async fn save_secrets(p: &RdsPasswords, re: RdsEndpoint, client: &secretsmanager::Client) -> Result<(), Error> {
    let sm_app = format!("{}-{}-application-conn-string", re.app_tag, re.db_identifier);
    let sm_mig = format!("{}-{}-migration-conn-string", re.app_tag, re.db_identifier);
    let sm_dbo = format!("{}-{}-dbowner-conn-string", re.app_tag, re.db_identifier);

    println!("Saving generated secrets for application, migration and dbowner");
    client.create_secret().name(sm_app).secret_string(&p.application).send().await?;
    client.create_secret().name(sm_mig).secret_string(&p.migration).send().await?;
    client.create_secret().name(sm_dbo).secret_string(&p.dbowner).send().await?;
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        // disable printing the name of the module in every log line.
        .with_target(false)
        // disabling time is handy because CloudWatch will add the ingestion time.
        .without_time()
        .init();

    run(service_fn(function_handler)).await
}
