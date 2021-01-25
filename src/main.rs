#![feature(destructuring_assignment)]
extern crate charts;
extern crate chrono;
extern crate itertools;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;

use std::error::Error;
use std::ffi::OsString;
use std::fs::{self, File};
use std::io;
use std::process::{Command, Output};
use std::str;

use charts::{Chart, Color, LineSeriesView, MarkerType, PointLabelPosition, ScaleBand, ScaleLinear};
use chrono::{Datelike, NaiveDate, NaiveDateTime};
use itertools::Itertools;

#[derive(Debug, Deserialize)]
struct User {
    name: String,
    muted: bool,
    id: i64,
}

#[derive(Debug, Deserialize)]
struct Chat {
    id: i64,
    pinned_message_id: Option<i64>,
    users: Vec<User>,
}

#[derive(Debug, Deserialize)]
struct Root {
    chats: Vec<Chat>,
    group_message_id: i64,
    groups: Vec<String>,
    hhh_id: Option<String>,
    main_id: Option<String>,
    recent_changes: Vec<String>,
}

static DATE_FORMAT: &str = "%d.%m.%Y";
static SVG_NAME: &str = "chart.svg";
static OUTPUT_NAME: &str = "chart.png";
static BASE_DIRECTORY: &str = "/home/chabare/state_backups/";
static CHART_TITLE: &str = "Number of chats";
static PICTURE_SIZE: (usize, usize) = (1920, 1080);

fn date_from_filename(filename: &str) -> Option<NaiveDateTime> {
    let name = filename.rsplit(".").last()?;

    let date = NaiveDateTime::from_timestamp(name.parse::<i64>().ok()?, 0);
    Some(NaiveDate::from_ymd(date.year(), date.month(), date.day()).and_hms(0, 0, 0))
}

fn parse_file((fileos, time): (OsString, NaiveDateTime)) -> Option<(String, u32)> {
    let filepath = String::from(BASE_DIRECTORY) + fileos.to_str()?;
    let file = File::open(filepath.clone()).ok()?;
    let root: Root = serde_json::from_reader(file).ok()?;

    Some((time.format(DATE_FORMAT).to_string(), root.chats.len() as u32))
}

fn create_bar_chart(data: Vec<(String, f32)>, filename: &str) -> Result<(), String> {
    let step_size = 1;

    let (dates, chat_lengths): (Vec<String>, Vec<f32>) = data.clone().into_iter().step_by(step_size).unzip();
    let label_offset = (12, 45);

    let width = PICTURE_SIZE.0 as isize;
    let height = PICTURE_SIZE.1 as isize;
    let (top, right, bottom, left) = (90, 40, 50 + (label_offset.1 as isize), 60);

    let date_scale = ScaleBand::new()
        .set_domain(dates.clone())
        .set_range(vec![0, width - left - right])
        .set_inner_padding(0.1)
        .set_outer_padding(0.1);

    let chat_length_scale = ScaleLinear::new()
        .set_domain(vec![chat_lengths.first().unwrap() - 10_f32, chat_lengths.get(chat_lengths.len() - 1).unwrap() + 10_f32])
        .set_range(vec![height - top - bottom, 0]);

    let view = LineSeriesView::new()
        .set_x_scale(&date_scale)
        .set_y_scale(&chat_length_scale)
        .set_marker_type(MarkerType::Circle)
        .set_label_position(PointLabelPosition::N)
        .set_colors(Color::color_scheme_light())
        .load_data(&data).unwrap();

    Chart::new()
        .set_width(width)
        .set_height(height)
        .set_margins(top, right, bottom, left)
        .add_title(String::from(CHART_TITLE))
        .add_view(&view)
        .add_axis_bottom(&date_scale, Some(label_offset))
        .add_axis_left(&chat_length_scale, None)
        .set_bottom_axis_tick_label_rotation(90)
        .save(filename)
}

fn convert(filename: &str, filename_output: &str) -> io::Result<Output> {
    Command::new("convert")
        .arg("-resize")
        .arg(&format!("{}x{}", PICTURE_SIZE.0, PICTURE_SIZE.1 + 70))
        .arg("-density")
        .arg("600")
        .arg("-background")
        .arg("#111")
        .arg(filename)
        .arg(filename_output)
        .output()
}

fn main() -> Result<(), Box<dyn Error>> {
    let result = fs::read_dir(BASE_DIRECTORY)?
        .filter_map(|file|
            Some(file.ok()?.file_name())
        )
        .filter_map(|s|
            Some((s.clone(), date_from_filename(s.to_str()?)?)))
        .sorted_by_key(|(_, d)| *d)
        .filter_map(parse_file)
        .unique_by(|(x, _)| x.clone())
        .fold(vec![], |mut a: Vec<(String, f32)>, x: (String, u32)| {
            // don't have adjacent numbers (a[i] == a[i + 1]), useless for the graph
            if a.last().unwrap_or(&("".to_string(), 0_f32)).1 != (x.1 as f32) {
                a.push((x.0, x.1 as f32))
            }

            a
        });

    create_bar_chart(result, SVG_NAME)?;
    let result = convert(SVG_NAME, OUTPUT_NAME)?;
    if result.stderr.len() > 0 || result.stdout.len() > 0 {
        println!("{} | {}", str::from_utf8(&result.stdout)?, str::from_utf8(&result.stderr)?);
    }

    Ok(())
}
