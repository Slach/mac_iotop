use std::collections::{HashMap, HashSet};
use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};
use std::time::Instant;

/// Интервал агрегации в секундах
const INTERVAL_SECS: u64 = 1;

fn main() {
    let debug = std::env::var("DEBUG").is_ok();

    let mut child = match Command::new("fs_usage")
        .args(["-w", "-f", "filesys"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Error: failed to run fs_usage: {}", e);
            eprintln!("Make sure you run with sudo: sudo mac_iotop");
            std::process::exit(1);
        }
    };

    // Проверяем, не завершился ли fs_usage сразу (например, без sudo)
    std::thread::sleep(std::time::Duration::from_millis(100));
    if let Some(status) = child.try_wait().unwrap_or(None) {
        if !status.success() {
            if let Some(mut stderr) = child.stderr.take() {
                let mut msg = String::new();
                std::io::Read::read_to_string(&mut stderr, &mut msg).ok();
                let msg = msg.trim();
                if !msg.is_empty() {
                    eprintln!("Error: fs_usage failed: {}", msg);
                } else {
                    eprintln!("Error: fs_usage exited with {}", status);
                }
            }
            eprintln!("Make sure you run with sudo: sudo mac_iotop");
            std::process::exit(1);
        }
    }

    let stdout = child.stdout.take().unwrap();
    let reader = BufReader::new(stdout);

    let mut last_time = String::new();
    // path -> (read_bytes, write_bytes, set<process_name>)
    let mut stats: HashMap<String, (u64, u64, HashSet<String>)> = HashMap::new();
    let mut interval_start = Instant::now();

    print_header();

    for line in reader.lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => continue,
        };

        let line = collapse_spaces(&line);

        let is_read = line.contains("RdData");
        let is_write = line.contains("WrData");
        if !is_read && !is_write {
            continue;
        }

        let time_str = match find_time(&line) {
            Some(t) => t,
            None => continue,
        };

        // Если секунда сменилась — проверяем интервал
        if time_str != last_time {
            if !last_time.is_empty() {
                let elapsed = interval_start.elapsed().as_secs();
                if elapsed >= INTERVAL_SECS {
                    let duration = elapsed.max(1) as f64;
                    print_stats(&last_time, &stats, duration, debug);
                    stats.clear();
                    interval_start = Instant::now();
                }
            }
            last_time = time_str;
        }

        let bytes = match line.find("B=0x") {
            Some(idx) => {
                let hex_str = line[idx + 4..].split_whitespace().next().unwrap_or("0");
                u64::from_str_radix(hex_str, 16).unwrap_or(0)
            }
            None => continue,
        };
        if bytes == 0 {
            continue;
        }

        let proc_name = line.split_whitespace().last().unwrap_or("?");

        let path = match extract_path(&line) {
            Some(p) if !p.is_empty() => p,
            _ => continue,
        };

        let entry = stats
            .entry(path)
            .or_insert_with(|| (0, 0, HashSet::new()));
        if is_read {
            entry.0 += bytes;
        } else {
            entry.1 += bytes;
        }
        entry.2.insert(proc_name.to_string());
    }

    // Остатки
    if !last_time.is_empty() && !stats.is_empty() {
        let duration = interval_start.elapsed().as_secs().max(1) as f64;
        print_stats(&last_time, &stats, duration, debug);
    }
}

fn print_header() {
    println!(
        "{:<10} | {:<10} | {:<10} | {}",
        "TIME", "READ/s", "WRITE/s", "PROCESS: FILE"
    );
    println!("{:-<90}", "");
}

fn print_stats(
    time: &str,
    stats: &HashMap<String, (u64, u64, HashSet<String>)>,
    duration: f64,
    debug: bool,
) {
    let mut entries: Vec<_> = stats.iter().collect();
    entries.sort_by(|a, b| (b.1 .0 + b.1 .1).cmp(&(a.1 .0 + a.1 .1)));

    for (file, (r, w, procs)) in &entries {
        let read_rate = *r as f64 / duration;
        let write_rate = *w as f64 / duration;
        let procs_str: Vec<&str> = procs.iter().map(|s| s.as_str()).collect();
        let procs_joined = procs_str.join(",");

        println!(
            "{:<10} | {:<10} | {:<10} | {}: {}",
            time,
            format_rate(read_rate),
            format_rate(write_rate),
            procs_joined,
            file,
        );
    }

    if debug {
        eprintln!("[DEBUG] interval={:.1}s entries={}", duration, entries.len());
    }

    // Разделитель между интервалами для читаемости
    let _ = std::io::stdout().flush();
}

fn format_rate(bytes_per_sec: f64) -> String {
    if bytes_per_sec == 0.0 {
        return "0".to_string();
    }
    const KB: f64 = 1024.0;
    const MB: f64 = 1024.0 * 1024.0;
    const GB: f64 = 1024.0 * 1024.0 * 1024.0;

    if bytes_per_sec >= GB {
        format!("{:.2} GB/s", bytes_per_sec / GB)
    } else if bytes_per_sec >= MB {
        format!("{:.2} MB/s", bytes_per_sec / MB)
    } else if bytes_per_sec >= KB {
        format!("{:.2} KB/s", bytes_per_sec / KB)
    } else {
        format!("{:.0} B/s", bytes_per_sec)
    }
}

fn collapse_spaces(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut prev_space = false;
    for c in s.chars() {
        if c == ' ' {
            if !prev_space {
                result.push(' ');
            }
            prev_space = true;
        } else {
            prev_space = false;
            result.push(c);
        }
    }
    result
}

fn find_time(line: &str) -> Option<String> {
    let bytes = line.as_bytes();
    if bytes.len() < 8 {
        return None;
    }
    for i in 0..=bytes.len() - 8 {
        if bytes[i + 2] == b':'
            && bytes[i + 5] == b':'
            && bytes[i].is_ascii_digit()
            && bytes[i + 1].is_ascii_digit()
            && bytes[i + 3].is_ascii_digit()
            && bytes[i + 4].is_ascii_digit()
            && bytes[i + 6].is_ascii_digit()
            && bytes[i + 7].is_ascii_digit()
        {
            return Some(line[i..i + 8].to_string());
        }
    }
    None
}

fn extract_path(line: &str) -> Option<String> {
    let dev_idx = line.find("/dev/disk")?;
    let after_dev = &line[dev_idx..];

    let space_idx = after_dev.find(' ')?;
    let after_dev_name = after_dev[space_idx..].trim_start();

    if after_dev_name.is_empty() || after_dev_name.as_bytes()[0].is_ascii_digit() {
        return None;
    }

    let pbytes = after_dev_name.as_bytes();
    let mut dur_pos = None;
    for i in (1..pbytes.len().saturating_sub(3)).rev() {
        if pbytes[i - 1] == b' '
            && pbytes[i].is_ascii_digit()
            && pbytes[i + 1] == b'.'
            && pbytes[i + 2].is_ascii_digit()
        {
            dur_pos = Some(i - 1);
            break;
        }
    }

    let path = match dur_pos {
        Some(pos) => after_dev_name[..pos].trim(),
        None => after_dev_name.trim(),
    };

    if path.is_empty() {
        return None;
    }

    if path.starts_with("private/") {
        Some(format!("/{}", path))
    } else {
        Some(path.to_string())
    }
}
