use serde::Serialize;
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};
use chrono::{DateTime, Utc};
use ureq;
use std::sync::{Arc, Mutex};

#[derive(Debug, Serialize)]
struct WebsiteStatus {
    url: String,
    status: Result<u16, String>,
    response_time: Duration,
    timestamp: DateTime<Utc>,
}

fn check_website(url: &str, timeout: Duration) -> WebsiteStatus {
    let start_time = Instant::now();
    let timestamp = Utc::now();

    let result = match ureq::get(url).timeout(timeout).call() {
        Ok(response) => Ok(response.status()),
        Err(err) => Err(err.to_string()),
    };

    let response_time = start_time.elapsed();

    WebsiteStatus {
        url: url.to_string(),
        status: result,
        response_time,
        timestamp,
    }
}

fn website_checker(
    urls: Vec<String>,
    worker_count: usize,
    timeout: Duration,
) -> Vec<WebsiteStatus> {
    let (tx, rx) = mpsc::channel();
    let urls = Arc::new(Mutex::new(urls));

    let mut threads = vec![];
    for _ in 0..worker_count {
        let tx = tx.clone();
        let urls = Arc::clone(&urls);
        threads.push(thread::spawn(move || {
            while let Ok(mut guard) = urls.lock() {
                if let Some(url) = guard.pop() {
                    drop(guard); // Release lock before performing HTTP request
                    let status = check_website(&url, timeout);
                    tx.send(status).expect("Failed to send status");
                } else {
                    break;
                }
            }
        }));
    }

    drop(tx); // Close sender

    for thread in threads {
        thread.join().expect("Thread panicked");
    }

    let mut results = vec![];
    for status in rx {
        results.push(status);
    }

    results
}

fn main() {
    let websites = vec![
        "https://www.rust-lang.org/".to_string(),
        "https://www.google.com/".to_string(),
        "https://www.github.com/".to_string(),
        "https://www.example.com/".to_string(),
    ];

    let worker_count = 4;
    let timeout = Duration::from_secs(5);

    let results = website_checker(websites, worker_count, timeout);

    for result in results {
        match &result.status {
            Ok(code) => println!(
                "{} - Status: {} - Time: {:?} - Timestamp: {}",
                result.url, code, result.response_time, result.timestamp
            ),
            Err(err) => println!(
                "{} - Error: {} - Time: {:?} - Timestamp: {}",
                result.url, err, result.response_time, result.timestamp
            ),
        }
    }
}
