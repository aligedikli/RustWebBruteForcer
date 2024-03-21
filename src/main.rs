use reqwest::Client;
use std::sync::Arc;
use tokio::sync::Semaphore;
use futures_util::stream::{self, StreamExt};
use std::io::{self, Write};

trait ScanTarget {
    fn url(&self) -> String;
}

struct Directory {
    base_url: String,
    path: String,
}

impl ScanTarget for Directory {
    fn url(&self) -> String {
        format!("{}/{}", self.base_url, self.path)
    }
}

struct Scanner {
    client: Client,
    concurrency_limit: usize,
}

impl Scanner {
    async fn scan<T: ScanTarget + Send + Sync + 'static>(&self, targets: Vec<T>) {
        let semaphore = Arc::new(Semaphore::new(self.concurrency_limit));
        stream::iter(targets).for_each_concurrent(None, |target| {
            let client = self.client.clone();
            let permit = semaphore.clone();
            async move {
                let _permit = permit.acquire().await;
                match client.get(target.url()).send().await {
                    Ok(response) => {
                        if response.status().is_success() {
                            println!("Found: {}", target.url());
                        }
                    },
                    Err(e) => eprintln!("Error scanning {}: {}", target.url(), e),
                }
            }
        }).await;
    }

    fn new(concurrency_limit: usize) -> Self {
        Scanner {
            client: Client::new(),
            concurrency_limit,
        }
    }
}
// author: https://github.com/aligedikli/RustWebBruteForcer 
#[tokio::main]
async fn main() {
    println!("Enter the base URL:");
    let mut base_url = String::new();
    io::stdin().read_line(&mut base_url).unwrap();
    base_url = base_url.trim().to_string(); // Trim the newline character at the end

    println!("Enter the paths separated by commas (e.g., admin,login,register):");
    let mut paths = String::new();
    io::stdin().read_line(&mut paths).unwrap();
    let paths: Vec<String> = paths.trim().split(',').map(|s| s.trim().to_string()).collect();

    let scanner = Scanner::new(5); // Limit concurrency to 5 requests at a time
    let targets = paths.into_iter().map(|path| {
        Directory { base_url: base_url.clone(), path }
    }).collect::<Vec<Directory>>();

    scanner.scan(targets).await;
}
