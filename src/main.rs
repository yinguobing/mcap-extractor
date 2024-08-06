use clap::Parser;
use indicatif::{ProgressBar, ProgressStyle};
use std::collections::HashMap;
use std::fs;
use std::io::Read;
use std::path::PathBuf;
use std::time::Duration;
mod h264;
mod vehicle;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// MCAP file to be parsed
    #[arg(short, long)]
    input: PathBuf,

    /// Output directory path
    #[arg(short, long)]
    output_dir: Option<PathBuf>,

    /// H264 topic
    #[arg(long)]
    topic: Option<String>,

    /// Camera sequence
    #[arg(long)]
    cam_seq: Option<String>,

    /// Verbose mode. This will log MCAP file summary and each processing steps.
    #[arg(long)]
    verbose: bool,
}

struct Topic {
    id: u16,
    name: String,
    description: String,
    msg_count: u64,
}

impl std::fmt::Display for Topic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "id: {}, {}, msgs: {}, {}",
            self.id, self.name, self.msg_count, self.description
        )
    }
}

fn summary(files: &Vec<PathBuf>) -> Vec<Topic> {
    let mut topics: HashMap<u16, Topic> = HashMap::new();

    for file in files {
        let mut fd = fs::File::open(file).unwrap();
        let mut buf = Vec::new();
        fd.read_to_end(&mut buf).unwrap();
        let Some(summary) = mcap::read::Summary::read(&buf).unwrap() else {
            panic!("Failed to read summary info: {}", file.display());
        };

        // Statistics
        let Some(stats) = summary.stats else {
            panic!("Failed to get statistics: {}", file.display())
        };

        // Topics
        let channels: Vec<_> = summary.channels.into_iter().collect();
        for chn in channels {
            topics
                .entry(chn.0)
                .and_modify(|t| {
                    t.id = chn.0;
                    t.name = chn.1.topic.clone();
                    t.description = format!(
                        "MsgType: {}, Encoding: {}",
                        chn.1.schema.as_ref().unwrap().name,
                        chn.1.schema.as_ref().unwrap().encoding
                    );
                    t.msg_count += stats.channel_message_counts.get(&chn.0).unwrap();
                })
                .or_insert(Topic {
                    id: chn.0,
                    name: chn.1.topic.clone(),
                    description: format!(
                        "MsgType: {}, Encoding: {}",
                        chn.1.schema.as_ref().unwrap().name,
                        chn.1.schema.as_ref().unwrap().encoding
                    ),
                    msg_count: stats.channel_message_counts.get(&chn.0).unwrap().clone(),
                });
        }
    }
    topics.into_values().collect()
}

fn main() {
    let args = Args::parse();

    // Safety first
    if !args.input.exists() {
        println!("Directory not found: {:?}", args.input.as_os_str());
        return;
    }

    // Find all macp files
    println!("Reading direcory...");
    let mut files: Vec<PathBuf> = fs::read_dir(&args.input)
        .unwrap()
        .map(|f| f.unwrap().path())
        .filter(|f| f.is_file() && f.extension().is_some_and(|f| f.eq("mcap")))
        .collect();
    files.sort();
    println!("Found MCAP files: {}", files.len());
    for f in files.iter() {
        println!("- {}", f.display());
    }

    // Summary these files
    let mut topics = summary(&files);
    topics.sort_by_key(|k| k.id);
    println!("Found topics: {}", topics.len());
    for topic in topics.iter() {
        println!("- {}", topic);
    }

    // Setup a progress bar.
    let spinner_style = ProgressStyle::default_spinner()
        .template("{prefix:} {spinner} {wide_msg}")
        .unwrap();
    let bar = ProgressBar::new_spinner();
    bar.set_style(spinner_style);
    bar.enable_steady_tick(Duration::from_millis(100));

    // Any extracting jobs?
    if let Some(topic) = args.topic {
        println!("Extracting: {}", topic);
        // Create output directory. Using topic name as directory path.
        let mut output_dir = args
            .output_dir
            .clone()
            .unwrap_or(std::env::current_dir().unwrap());
        let sub_dir = PathBuf::from(topic.trim_start_matches('/'));
        output_dir.push(sub_dir);
        if fs::create_dir_all(&output_dir).is_err() {
            println!("Failed to create output directory: {:?}", output_dir);
            return;
        };
        println!("- Output directory: {:}", output_dir.display());

        println!("- Extracing H.264...");
        let mut parser = h264::Parser::new(&output_dir);
        let mut counter = 0;
        for file in files.iter() {
            let mut fd = fs::File::open(file).unwrap();
            let mut buf = Vec::new();
            fd.read_to_end(&mut buf).unwrap();
            let stream = mcap::MessageStream::new(&buf)
                .unwrap()
                .filter(|x| x.as_ref().is_ok_and(|x| x.channel.topic == topic));
            for message in stream {
                let message = message.unwrap();
                parser.accumulate(&message);
                counter += 1;
                bar.set_message(format!("{} {}", topic, counter.to_string()));
            }
        }
        bar.finish();

        // Dump frames
        println!("- Extracing camera frames...");
        parser.dump_frames();
    } else {
        println!("No topics specified.");
    }
    println!("Done.");
}
