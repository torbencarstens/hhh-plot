extern crate charts;
extern crate chrono;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;

use std::error::Error;
use std::ffi::OsString;
use std::fs::{self, File};
use std::io;
use std::io::Read;
use std::process::{Command, Output};
use std::str;

use charts::{Chart, Color, LineSeriesView, MarkerType, PointLabelPosition, ScaleBand, ScaleLinear};
use chrono::{Datelike, NaiveDate, NaiveDateTime};

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

    Some(NaiveDateTime::from_timestamp(name.parse::<i64>().ok()?, 0))

    // Some(NaiveDate::from_isoywd(timestamp.year(), timestamp.iso_week().week(), timestamp.weekday()))
}

fn parse_file((fileos, _): (OsString, NaiveDateTime)) -> Option<(NaiveDateTime, Root)> {
    let filepath = String::from(BASE_DIRECTORY) + fileos.to_str()?;
    let mut file = File::open(filepath.clone()).ok()?;
    let mut content = String::new();
    file.read_to_string(&mut content).ok()?;
    let root: Root = serde_json::from_str(&*content).ok()?;

    Some((date_from_filename(fileos.to_str()?)?, root))
}

fn dedup(xseries: Vec<String>, yseries: Vec<f32>) -> (Vec<String>, Vec<f32>) {
    let mut xs = vec![];
    let mut ys = vec![];

    for (i, x) in xseries.iter().enumerate() {
        if xs.iter().filter(|&n| *n == *x).count() == 0 {
            ys.push(*yseries.get(i).unwrap());
            xs.push(xseries.get(i).unwrap().to_string())
        }
    }

    let mut xs2 = vec![];
    let mut ys2 = vec![];

    for (i, y) in ys.iter().enumerate() {
        if ys2.iter().filter(|&n| *n == *y).count() == 0 {
            ys2.push(*ys.get(i).unwrap());
            xs2.push(xs.get(i).unwrap().to_string())
        }
    }

    (xs2, ys2)
}

fn create_bar_chart(xseries: Vec<String>, yseries: Vec<f32>, filename: &str) -> Result<(), String> {
    let step_size = 1;
    let yseries = yseries.into_iter().step_by(step_size).collect::<Vec<f32>>();
    let xseries = xseries.into_iter().step_by(step_size).collect::<Vec<String>>();

    let width = PICTURE_SIZE.0 as isize;
    let height = PICTURE_SIZE.1 as isize;
    let (top, right, bottom, left) = (90, 40, 50, 60);

    let x = ScaleBand::new()
        .set_domain(xseries.clone())
        .set_range(vec![0, width - left - right])
        .set_inner_padding(0.1)
        .set_outer_padding(0.1);

    let y = ScaleLinear::new()
        .set_domain(vec![yseries.first().unwrap() - 10_f32, yseries.get(yseries.len() - 1).unwrap() + 10_f32])
        .set_range(vec![height - top - bottom, 0]);

    let data = xseries.into_iter().zip(yseries).collect();

    let view = LineSeriesView::new()
        .set_x_scale(&x)
        .set_y_scale(&y)
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
        .add_axis_bottom(&x, Some((45, 12)))
        .add_axis_left(&y, None)
        .set_bottom_axis_tick_label_rotation(90)
        .save(filename)
}

fn convert(filename: &str, filename_output: &str) -> io::Result<Output> {
    Command::new("convert")
        .arg("-resize")
        .arg(&format!("{}x{}", PICTURE_SIZE.0, PICTURE_SIZE.1))
        .arg("-density")
        .arg("600")
        .arg("-background")
        .arg("#111")
        .arg(filename)
        .arg(filename_output)
        .output()
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut result = fs::read_dir(BASE_DIRECTORY)?
        .filter_map(|file| {
            let filename = file.ok()?.file_name();
            Some((filename.clone(), date_from_filename(filename.to_str()?)?))
        })
        .collect::<Vec<(OsString, NaiveDateTime)>>()
        .into_iter()
        .filter_map(parse_file)
        .collect::<Vec<(NaiveDateTime, Root)>>();
    result.sort_by_key(|(d, _)|
        NaiveDate::from_ymd(d.year(), d.month(), d.day()).and_hms(0, 0, 0));
    let xs = result.iter().map(|(date, _)|
        date.format(DATE_FORMAT).to_string()).collect::<Vec<String>>();
    let ys = result.iter().map(|(_, root)| root.chats.len() as f32).collect::<Vec<f32>>();

    let (xs, ys) = dedup(xs, ys);

    create_bar_chart(xs, ys, SVG_NAME)?;
    let result = convert(SVG_NAME, OUTPUT_NAME)?;
    if result.stderr.len() > 0 || result.stdout.len() > 0 {
        println!("{} | {}", str::from_utf8(&result.stdout)?, str::from_utf8(&result.stderr)?);
    }

    Ok(())
}
