use clap::{App, Arg, SubCommand};
use chrono::Local;
use csv::{Reader, Writer, Error};
use std::fs::{OpenOptions, File};
use std::io::{self, Read, Write};
use std::path::Path;
use chrono::NaiveDateTime;
use chrono::TimeZone;

use directories::ProjectDirs;
use std::path::PathBuf;

// ...



fn main() -> Result<(), Error> {
    let matches = App::new("Job Hours Logger")
        .version("1.0")
        .author("Your Name <your.email@example.com>")
        .about("Logs the hours spent on your job")
        .subcommand(SubCommand::with_name("clock_in").about("Clock in for work"))
        .subcommand(SubCommand::with_name("clock_out").about("Clock out from work"))
        .subcommand(SubCommand::with_name("summary").about("Show work hours summary"))
        .subcommand(SubCommand::with_name("edit_start").about("Edit the start time of the last recorded shift").arg(
            Arg::with_name("new_start_time")
                .help("New start time in the format: %Y-%m-%d %H:%M:%S")
                .required(true),
        ))
        .subcommand(SubCommand::with_name("edit_stop").about("Edit the stop time of the last recorded shift").arg(
            Arg::with_name("new_stop_time")
                .help("New stop time in the format: %Y-%m-%d %H:%M:%S")
                .required(true),
        ))
        .get_matches();

    if let Some(matches) = matches.subcommand_matches("edit_start") {
        let new_start_time = matches.value_of("new_start_time").unwrap();
        edit_start(new_start_time)?;
    } else if let Some(matches) = matches.subcommand_matches("edit_stop") {
        let new_stop_time = matches.value_of("new_stop_time").unwrap();
        edit_stop(new_stop_time)?;
    }
    else if let Some(_) = matches.subcommand_matches("clock_in") {
        clock_in()?;
    } else if let Some(_) = matches.subcommand_matches("clock_out") {
        clock_out()?;
    } else if let Some(_) = matches.subcommand_matches("summary") {
        print_summary()?;
    } else {
        println!("Please use 'clock_in', 'clock_out', 'edit_start', 'edit_stop' or 'summary' subcommand.");
    }
// ...


    Ok(())
}

fn clock_in() -> Result<(), Error> {
    let now = Local::now();
    let csv_path = get_csv_path();
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(csv_path)?;
    writeln!(file, "{}", now.to_rfc3339())?;
    println!("Clocked in at {}", now.format("%Y-%m-%d %H:%M:%S"));
    Ok(())
}

fn clock_out() -> Result<(), Error> {
    let csv_path = get_csv_path();
    let last_clock_in = find_last_clock_in()?;

    if let Some(clock_in_time) = last_clock_in {
        let now = Local::now();
        let duration = now - clock_in_time;

        let hours = duration.num_minutes() as f64 / 60.0;
        println!(
            "Clocked out at: {}\nTotal hours worked: {:.2}",
            now.format("%Y-%m-%d %H:%M:%S"),
            hours
        );

        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(csv_path)?;

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


fn find_last_clock_in() -> Result<Option<chrono::DateTime<Local>>, Error> {
    let csv_path = get_csv_path();
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(csv_path)?;

    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    let last_line = contents.lines().last();

    if let Some(last_line) = last_line {
        let last_clock_in = chrono::DateTime::parse_from_rfc3339(last_line)
            .map_err(|e| csv::Error::from(io::Error::new(io::ErrorKind::Other, e)))?;
        Ok(Some(last_clock_in.with_timezone(&Local)))
    } else {
        Ok(None)
    }
}

fn create_or_get_csv_writer() -> Result<csv::Writer<File>, Error> {
    let csv_path = get_csv_path();
    let file_path = csv_path.as_path();

    if !file_path.exists() {
        let mut csv_writer = Writer::from_path(file_path)?;
        csv_writer.write_record(&["Clock In", "Clock Out", "Hours"])?;
    }

    let file = OpenOptions::new().append(true).open(file_path)?;
    Ok(Writer::from_writer(file))
}

fn print_summary() -> Result<(), Error> {
    let csv_path = get_csv_path();
    let file = File::open(csv_path)?;
    let mut csv_reader = Reader::from_reader(file);
    let mut total_hours = 0.0;

    println!("Start Time\t\t\tEnd Time\t\t\tHours");
    println!("------------------------------------------------------------");

    for record in csv_reader.records() {
        let record = record?;
        let start_time = &record[0];
        let end_time = &record[1];
        let hours: f64 = record[2].parse().unwrap_or(0.0);
        total_hours += hours;

        println!("{}\t{}\t{:.2}", start_time, end_time, hours);
    }

    println!("------------------------------------------------------------");
    println!("Total Hours Worked: {:.2}", total_hours);

    Ok(())
}

fn edit_start(new_start_time: &str) -> Result<(), Error> {
    let new_start_time = parse_datetime(new_start_time)?;
    update_last_shift_record(0, new_start_time)?;
    println!("Updated start time to: {}", new_start_time.format("%Y-%m-%d %H:%M:%S"));
    Ok(())
}

fn edit_stop(new_stop_time: &str) -> Result<(), Error> {
    let new_stop_time = parse_datetime(new_stop_time)?;
    update_last_shift_record(1, new_stop_time)?;
    println!("Updated stop time to: {}", new_stop_time.format("%Y-%m-%d %H:%M:%S"));
    Ok(())
}

fn parse_datetime(datetime_str: &str) -> Result<chrono::DateTime<Local>, Error> {
    let naive_dt = NaiveDateTime::parse_from_str(datetime_str, "%Y-%m-%d %H:%M:%S")
        .map_err(|e| csv::Error::from(io::Error::new(io::ErrorKind::Other, e)))?;
    Ok(Local.from_local_datetime(&naive_dt).unwrap())
}

fn update_last_shift_record(index: usize, new_time: chrono::DateTime<Local>) -> Result<(), Error> {
    let csv_path = get_csv_path();
    let file = File::open(csv_path)?;
    let mut csv_reader = Reader::from_reader(file);
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

    let mut csv_writer = Writer::from_path("job_hours_records.csv")?;
    csv_writer.write_record(&["Clock In", "Clock Out", "Hours"])?;


    for record in records {
        let start_time = chrono::DateTime::parse_from_rfc3339(&record[0])
            .map_err(|e| csv::Error::from(io::Error::new(io::ErrorKind::Other, e)))?;
        let end_time = chrono::DateTime::parse_from_rfc3339(&record[1])
            .map_err(|e| csv::Error::from(io::Error::new(io::ErrorKind::Other, e)))?;
        let duration = end_time - start_time;
        let hours = duration.num_minutes() as f64 / 60.0;
        csv_writer.write_record(&[&record[0], &record[1], &format!("{:.2}", hours)])?;
    }


    csv_writer.flush()?;

    Ok(())
}

fn get_csv_path() -> PathBuf {
    let project_dirs = ProjectDirs::from("com", "YourCompany", "YourAppName")
        .expect("Cannot determine the appropriate directories for this system.");

    let data_dir = project_dirs.data_local_dir();
    std::fs::create_dir_all(data_dir).expect("Cannot create the data directory.");

    data_dir.join("job_hours_records.csv")
}
