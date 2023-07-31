use chrono::DateTime;
use chrono::Local;
use chrono::NaiveDateTime;
use chrono::TimeZone;
use clap::{App, Arg, SubCommand};
use csv::{Error, ReaderBuilder, Writer};
use std::fs::{File, OpenOptions};
use std::io::{self, Write};

use directories::ProjectDirs;
use std::path::PathBuf;

// ...

fn main() -> Result<(), Error> {
    let matches = App::new("Job Hours Logger")
        .version("1.0")
        .author("Grayson Hieb <grayson.hieb@du.edu>")
        .about("Logs the hours spent on your jobs")
        .arg(
            Arg::with_name("job")
                .help("The name of the job")
                .global(true),
        )
        .subcommand(SubCommand::with_name("clock_in").about("Clock in for work"))
        .subcommand(SubCommand::with_name("clock_out").about("Clock out from work"))
        .subcommand(SubCommand::with_name("summary").about("Show work hours summary"))
        .subcommand(
            SubCommand::with_name("edit_start")
                .about("Edit the start time of the last recorded shift")
                .arg(
                    Arg::with_name("new_start_time")
                        .help("New start time in the format: %Y-%m-%d %H:%M:%S")
                        .required(true),
                ),
        )
        .subcommand(
            SubCommand::with_name("edit_stop")
                .about("Edit the stop time of the last recorded shift")
                .arg(
                    Arg::with_name("new_stop_time")
                        .help("New stop time in the format: %Y-%m-%d %H:%M:%S")
                        .required(true),
                ),
        )
        .get_matches();

    let job_name = matches.value_of("job").unwrap_or_default();

    if let Some(matches) = matches.subcommand_matches("edit_start") {
        let new_start_time = matches.value_of("new_start_time").unwrap();
        edit_start(job_name, new_start_time)?;
    } else if let Some(matches) = matches.subcommand_matches("edit_stop") {
        let new_stop_time = matches.value_of("new_stop_time").unwrap();
        edit_stop(job_name, new_stop_time)?;
    } else if let Some(_) = matches.subcommand_matches("clock_in") {
        clock_in(Some(job_name))?;
    } else if let Some(_) = matches.subcommand_matches("clock_out") {
        clock_out(job_name)?;
    } else if let Some(_) = matches.subcommand_matches("summary") {
        print_summary(job_name)?;
    } else {
        println!("Please use 'clock_in', 'clock_out', 'edit_start', 'edit_stop' or 'summary' subcommand.");
    }
    // ...

    Ok(())
}

fn clock_in(job: Option<&str>) -> Result<(), Error> {
    let now = Local::now();
    let job = job.unwrap_or("GenAI");
    let csv_path = get_csv_path(job);
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(csv_path)?;
    writeln!(file, "{},{}", job, now.to_rfc3339())?;
    println!(
        "Clocked in at {} for {}",
        now.format("%Y-%m-%d %H:%M:%S"),
        job
    );
    Ok(())
}

fn clock_out(job_name: &str) -> Result<(), Error> {
    let last_clock_in = find_last_clock_in(job_name)?;
    let csv_path = get_csv_path(job_name);
    let file = OpenOptions::new()
        .create(false)
        .append(true)
        .open(csv_path)?;
    println!("Here");

    if let Some(clock_in_time) = last_clock_in {
        let now = Local::now();
        let duration = now - clock_in_time;

        let hours = duration.num_minutes() as f64 / 60.0;
        println!(
            "Clocked out at: {}\nTotal hours worked: {:.2}",
            now.format("%Y-%m-%d %H:%M:%S"),
            hours
        );

        let mut csv_writer = Writer::from_writer(file);
        csv_writer.write_record(&[
            clock_in_time.to_rfc3339(),
            now.to_rfc3339(),
            format!("{:.2}", hours),
        ])?;

        csv_writer.flush()?;
    } else {
        println!("No clock in entry found. Please clock in first.");
    }

    Ok(())
}

fn find_last_clock_in(job_name: &str) -> Result<Option<DateTime<Local>>, Error> {
    let csv_path = get_csv_path(job_name);
    let file = File::open(csv_path)?;
    let mut reader = ReaderBuilder::new().flexible(true).from_reader(file);

    let mut last_clock_in: Option<DateTime<Local>> = None;
    for result in reader.records() {
        let record = result?;
        println!("Record: {:?}", record);
        if record.len() == 2 {
            let clock_in_str = &record[1];
            println!("Last clock in {}", clock_in_str);
            last_clock_in = Some(
                DateTime::parse_from_rfc3339(clock_in_str)
                    .map_err(|e| csv::Error::from(io::Error::new(io::ErrorKind::Other, e)))?
                    .with_timezone(&Local),
            );
        }
    }

    Ok(last_clock_in)
}

fn print_summary(job_name: &str) -> Result<(), Error> {
    let csv_path = get_csv_path(job_name);
    let file = File::open(csv_path)?;
    let mut csv_reader = ReaderBuilder::new().flexible(true).from_reader(file);
    let mut total_hours = 0.0;

    println!("Start Time\t\t\tEnd Time\t\t\tHours");
    println!("------------------------------------------------------------");

    for record in csv_reader.records() {
        let record = record?;
        if record.len() == 3 {
            let start_time = &record[0];
            let end_time = &record[1];
            let hours: f64 = record[2].parse().unwrap_or(0.0);
            total_hours += hours;

            println!("{}\t{}\t{:.2}", start_time, end_time, hours);
        }
    }

    println!("------------------------------------------------------------");
    println!("Total Hours Worked: {:.2}", total_hours);

    Ok(())
}

fn edit_start(job_name: &str, new_start_time: &str) -> Result<(), Error> {
    let new_start_time = parse_datetime(new_start_time)?;
    update_last_shift_record(job_name, 0, new_start_time)?;
    println!(
        "Updated start time to: {}",
        new_start_time.format("%Y-%m-%d %H:%M:%S")
    );
    Ok(())
}

fn edit_stop(job_name: &str, new_stop_time: &str) -> Result<(), Error> {
    let new_stop_time = parse_datetime(new_stop_time)?;
    update_last_shift_record(job_name, 1, new_stop_time)?;
    println!(
        "Updated stop time to: {}",
        new_stop_time.format("%Y-%m-%d %H:%M:%S")
    );
    Ok(())
}

fn parse_datetime(datetime_str: &str) -> Result<chrono::DateTime<Local>, Error> {
    let naive_dt = NaiveDateTime::parse_from_str(datetime_str, "%Y-%m-%d %H:%M:%S")
        .map_err(|e| csv::Error::from(io::Error::new(io::ErrorKind::Other, e)))?;
    Ok(Local.from_local_datetime(&naive_dt).unwrap())
}

fn update_last_shift_record(
    job_name: &str,
    index: usize,
    new_time: chrono::DateTime<Local>,
) -> Result<(), Error> {
    let csv_path = get_csv_path(job_name);
    let file = File::open(csv_path.clone())?;
    let mut csv_reader = ReaderBuilder::new().flexible(true).from_reader(file);
    let mut records: Vec<Vec<String>> = csv_reader
        .records()
        .map(|record| record.map(|r| r.iter().map(String::from).collect()))
        .collect::<Result<Vec<_>, _>>()?;

    let last_record_index = records.len().checked_sub(1).ok_or_else(|| {
        csv::Error::from(io::Error::new(
            io::ErrorKind::Other,
            "No records found to update.",
        ))
    })?;

    records[last_record_index][index] = new_time.to_rfc3339();

    let mut csv_writer = Writer::from_path(csv_path)?;
    csv_writer.write_record(&["Job", "Clock In", "Clock Out", "Hours"])?;

    for record in records {
        if record.len() == 4 {
            let start_time = chrono::DateTime::parse_from_rfc3339(&record[1])
                .map_err(|e| csv::Error::from(io::Error::new(io::ErrorKind::Other, e)))?;
            let end_time = chrono::DateTime::parse_from_rfc3339(&record[2])
                .map_err(|e| csv::Error::from(io::Error::new(io::ErrorKind::Other, e)))?;
            let duration = end_time - start_time;
            let hours = duration.num_minutes() as f64 / 60.0;
            csv_writer.write_record(&[
                &record[0],
                &record[1],
                &record[2],
                &format!("{:.2}", hours),
            ])?;
        } else if record.len() == 1 {
            csv_writer.write_record(&[&record[0]])?;
        }
    }

    csv_writer.flush()?;

    Ok(())
}

fn get_csv_path(job: &str) -> PathBuf {
    let project_dirs = ProjectDirs::from("com", "Lowband", "HourTracker")
        .expect("Cannot determine the appropriate directories for this system.");

    let data_dir = project_dirs.data_local_dir();
    std::fs::create_dir_all(data_dir).expect("Cannot create the data directory.");

    data_dir.join(format!("{}_hours_records.csv", job))
}
