use anyhow::Result;
use futures::future::join_all;
use std::{
    path::PathBuf,
    process::{Child, Command},
    // sync::Arc,
    time::{Duration, Instant},
};
use thirtyfour::prelude::*;
// use tokio::sync::Mutex;

fn time_log(start: &Instant, tag: Option<&str>) {
    let end = start.elapsed();
    let mut text = format!("{}.{:03}秒", end.as_secs(), end.subsec_nanos() / 1000000).to_string();
    if let Some(tag) = tag {
        text = format!("{tag}: {text}").to_string();
    }
    println!("{text}");
}

fn get_driver(name: &str) -> Result<PathBuf> {
    let path = which::which(name)?;
    Ok(path)
}

fn start_chromedriver(port: &str) -> Result<Child> {
    let path = get_driver("chromedriver")?;
    let mut cmd = Command::new(path);
    cmd.arg(format!("--port={}", port));
    let child = cmd.spawn()?;
    Ok(child)
}
fn start_safaridriver(port: &str) -> Result<Child> {
    let path = get_driver("safaridriver")?;
    let mut cmd = Command::new(path);
    cmd.arg("--port").arg(port);
    let child = cmd.spawn()?;
    Ok(child)
}

#[async_trait::async_trait]
trait Driver {
    fn run(&self) -> Result<Child>;
    async fn build(&self) -> Result<WebDriver>;
}

#[derive(Clone)]
struct Safari {
    port: String,
}
#[async_trait::async_trait]
impl Driver for Safari {
    fn run(&self) -> Result<Child> {
        start_safaridriver(&self.port)
    }
    async fn build(&self) -> Result<WebDriver> {
        let caps = DesiredCapabilities::safari();
        let driver = WebDriver::new(
            format!("http://localhost:{}", self.port.as_str()).as_str(),
            caps,
        )
        .await?;
        Ok(driver)
    }
}
unsafe impl Send for Safari {}
unsafe impl Sync for Safari {}

#[derive(Clone)]
struct Chrome {
    port: String,
}
#[async_trait::async_trait]
impl Driver for Chrome {
    fn run(&self) -> Result<Child> {
        start_chromedriver(&self.port)
    }
    async fn build(&self) -> Result<WebDriver> {
        let mut caps = DesiredCapabilities::chrome();
        caps.add_chrome_arg("--headless")?;
        let driver = WebDriver::new(
            format!("http://localhost:{}", self.port.as_str()).as_str(),
            caps,
        )
        .await?;
        Ok(driver)
    }
}
unsafe impl Send for Chrome {}
unsafe impl Sync for Chrome {}

#[derive(Clone)]
struct A {
    index: usize,
    e: String,
}
impl std::fmt::Display for A {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.index, self.e)
    }
}

// #[derive(Clone)]
// struct As(Vec<A>);

// impl std::fmt::Display for As {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         let text = self
//             .0
//             .clone()
//             .into_iter()
//             .map(|i| format!("{}: {}", i.index, i.e).to_string())
//             .collect::<Vec<String>>()
//             .join(",");
//         write!(f, "{}", text)
//     }
// }
// impl std::convert::From<Vec<A>> for As {
//     fn from(value: Vec<A>) -> Self {
//         Self(value.clone())
//     }
// }

async fn browse(driver: WebDriver) -> Result<()> {
    let start = Instant::now();
    driver.goto("https://qiita.com/").await?;
    time_log(&start, Some("Opened Page"));
    tokio::time::sleep(Duration::from_secs(1)).await;
    let elem = driver.find_all(By::Tag("a")).await?;
    time_log(&start, Some("Got A Element"));
    let texts = elem.iter().map(|e| e.text());
    let res = join_all(texts).await;
    let res = res.into_iter().enumerate().for_each(|(i, r)| {
        if let Some(r) = r.ok() {
            let a = A {
                index: i,
                e: r.clone(),
            };
            println!("{}", a);
        };
    });
    // let res = res
    //     .into_iter()
    //     .enumerate()
    //     .filter_map(|(i, r)| {
    //         if let Some(r) = r.ok() {
    //             let a = A {
    //                 index: i,
    //                 e: r.clone(),
    //             };
    //             Some(a)
    //         } else {
    //             None
    //         }
    //     })
    //     .collect::<Vec<A>>();
    // println!("{}", As::from(res));
    driver.quit().await?;
    time_log(&start, Some("End"));
    Ok(())
}

async fn run<T: Driver + Sync + Send>(driver: T) -> Result<()> {
    let driver = driver.build().await?;
    match browse(driver).await {
        Err(e) => {
            println!("{e:?}");
        }
        _ => {}
    };
    Ok(())
}

// async fn run_<T: Driver + Sync + Send>(driver: Arc<Mutex<T>>) -> Result<()> {
//     let driver = driver.lock().await.build().await?;
//     match browse(driver).await {
//         Err(e) => {
//             println!("{e:?}");
//         }
//         _ => {}
//     };
//     Ok(())
// }

#[tokio::main]
async fn main() -> Result<()> {
    let start = Instant::now();
    let port = "4444";
    let driver = Chrome {
        port: port.to_string(),
    };
    let mut child = driver.run()?;
    // let driver = Arc::new(Mutex::new(driver));
    let handles = (0..50).map(|_| run(driver.clone())).collect::<Vec<_>>();
    join_all(handles).await;
    child.kill()?;
    time_log(&start, Some("End of all process"));
    Ok(())
}
