/* 
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: September 24, 2017
Last Modified: September 24, 2017
License: MIT
*/
extern crate time;
extern crate num_cpus;

use std::fs::File;
use std::io::prelude::*;
use std::process::Command;
use std::env;
use std::path;
use std::f64;
use lidar::*;
use std::io::{Error, ErrorKind};
use tools::WhiteboxTool;

pub struct LidarKappaIndex {
    name: String,
    description: String,
    parameters: String,
    example_usage: String,
}

impl LidarKappaIndex {
    pub fn new() -> LidarKappaIndex { // public constructor
        let name = "LidarKappaIndex".to_string();
        
        let description = "Performs a kappa index of agreement (KIA) analysis on the classifications of two LAS files.".to_string();
        
        let mut parameters = "--i1, --input1    Input LAS file (classification).".to_owned();
        parameters.push_str("--i2, --input2    Input LAS file (reference).\n");
        parameters.push_str("-o, --output     Output HTML file.\n");
        
        let sep: String = path::MAIN_SEPARATOR.to_string();
        let p = format!("{}", env::current_dir().unwrap().display());
        let e = format!("{}", env::current_exe().unwrap().display());
        let mut short_exe = e.replace(&p, "").replace(".exe", "").replace(".", "").replace(&sep, "");
        if e.contains(".exe") {
            short_exe += ".exe";
        }
        let usage = format!(">>.*{0} -r={1} --wd=\"*path*to*data*\" --i1=class.tif --i2=reference.tif -o=kia.html", short_exe, name).replace("*", &sep);
    
        LidarKappaIndex { name: name, description: description, parameters: parameters, example_usage: usage }
    }
}

impl WhiteboxTool for LidarKappaIndex {
    fn get_tool_name(&self) -> String {
        self.name.clone()
    }

    fn get_tool_description(&self) -> String {
        self.description.clone()
    }

    fn get_tool_parameters(&self) -> String {
        self.parameters.clone()
    }

    fn get_example_usage(&self) -> String {
        self.example_usage.clone()
    }

    fn run<'a>(&self, args: Vec<String>, working_directory: &'a str, verbose: bool) -> Result<(), Error> {
        let mut input_file1 = String::new();
        let mut input_file2 = String::new();
        let mut output_file = String::new();
         
        if args.len() == 0 {
            return Err(Error::new(ErrorKind::InvalidInput,
                                "Tool run with no paramters. Please see help (-h) for parameter descriptions."));
        }
        for i in 0..args.len() {
            let mut arg = args[i].replace("\"", "");
            arg = arg.replace("\'", "");
            let cmd = arg.split("="); // in case an equals sign was used
            let vec = cmd.collect::<Vec<&str>>();
            let mut keyval = false;
            if vec.len() > 1 {
                keyval = true;
            }
            if vec[0].to_lowercase() == "-i1" || vec[0].to_lowercase() == "--i1" || vec[0].to_lowercase() == "--input1" {
                if keyval {
                    input_file1 = vec[1].to_string();
                } else {
                    input_file1 = args[i+1].to_string();
                }
            } else if vec[0].to_lowercase() == "-i2" || vec[0].to_lowercase() == "--i2" || vec[0].to_lowercase() == "--input2" {
                if keyval {
                    input_file2 = vec[1].to_string();
                } else {
                    input_file2 = args[i+1].to_string();
                }
            } else if vec[0].to_lowercase() == "-o" || vec[0].to_lowercase() == "--output" {
                if keyval {
                    output_file = vec[1].to_string();
                } else {
                    output_file = args[i+1].to_string();
                }
            }
        }

        if verbose {
            println!("***************{}", "*".repeat(self.get_tool_name().len()));
            println!("* Welcome to {} *", self.get_tool_name());
            println!("***************{}", "*".repeat(self.get_tool_name().len()));
        }

        let sep: String = path::MAIN_SEPARATOR.to_string();

        let mut progress: i32;
        let mut old_progress: i32 = -1;

        if !input_file1.contains(&sep) {
            input_file1 = format!("{}{}", working_directory, input_file1);
        }
        if !input_file2.contains(&sep) {
            input_file2 = format!("{}{}", working_directory, input_file2);
        }
        if !output_file.contains(&sep) {
            output_file = format!("{}{}", working_directory, output_file);
        }
        if !output_file.ends_with(".html") {
            output_file = output_file + ".html";
        }

        if verbose { println!("Reading data...") };
        let start = time::now();

        let input1: LasFile = match LasFile::new(&input_file1, "r") {
            Ok(lf) => lf,
            Err(err) => panic!("Error: {}", err),
        };

        let input2: LasFile = match LasFile::new(&input_file2, "r") {
            Ok(lf) => lf,
            Err(err) => panic!("Error: {}", err),
        };

        let num_points = input1.header.number_of_points;
        if input2.header.number_of_points != num_points {
            panic!("Error: The input files do not contain the same number of points.");
        }
        let mut error_matrix: [[usize; 256]; 256] = [[0; 256]; 256];
        let mut active_class: [bool; 256] = [false; 256];
        let mut p1: PointData;
        let mut p2: PointData;
        let (mut class1, mut class2): (usize, usize);
        for i in 0..num_points as usize {
            p1 = input1.get_point_info(i);
            p2 = input2.get_point_info(i);
            class1 = p1.classification() as usize;
            class2 = p2.classification() as usize;
            error_matrix[class1][class2] += 1;
            active_class[class1] = true;
            active_class[class2] = true;

            if verbose {
                progress = (100.0_f64 * i as f64 / num_points as f64) as i32;
                if progress != old_progress {
                    println!("Progress: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        let mut num_classes = 0;
        for a in 0..256usize {
            if active_class[a] { num_classes += 1; }
        }

        let mut agreements = 0usize;
        let mut expected_frequency = 0f64;
        let mut n = 0usize;
        let mut row_total: usize;
        let mut col_total: usize;
        let kappa: f64;
        let overall_accuracy: f64;

        for a in 0..256usize {
            agreements += error_matrix[a][a];
            for b in 0..256usize {
                n += error_matrix[a][b];
            }
        }

        for a in 0..256usize {
            row_total = 0;
            col_total = 0;
            for b in 0..256usize {
                col_total += error_matrix[a][b];
                row_total += error_matrix[b][a];
            }
            expected_frequency += (col_total as f64 * row_total as f64) / (n as f64);
        }

        kappa = (agreements as f64 - expected_frequency as f64) / (n as f64 - expected_frequency as f64);
        overall_accuracy = agreements as f64 / n as f64;

        let mut f = File::create(output_file.as_str()).unwrap();

        let mut s = "<!DOCTYPE html PUBLIC \"-//W3C//DTD XHTML 1.0 Transitional//EN\" \"http://www.w3.org/TR/xhtml1/DTD/xhtml1-transitional.dtd\">
        <head>
            <meta content=\"text/html; charset=iso-8859-1\" http-equiv=\"content-type\">
            <title>Lidar Kappa Index of Agreement</title>
            <style  type=\"text/css\">
                h1 {
                    font-size: 14pt;
                    margin-left: 15px;
                    margin-right: 15px;
                    text-align: center;
                    font-family: Helvetica, Verdana, Geneva, Arial, sans-serif;
                }
                h3 {
                    font-size: 12pt;
                    margin-left: 15px;
                    margin-right: 15px;
                    text-align: left;
                    font-family: Helvetica, Verdana, Geneva, Arial, sans-serif;
                }
                p, ol, ul, li {
                    font-size: 12pt;
                    font-family: Helvetica, Verdana, Geneva, Arial, sans-serif;
                    margin-left: 15px;
                    margin-right: 15px;
                }
                caption {
                    font-family: Helvetica, Verdana, Geneva, Arial, sans-serif;
                    font-size: 12pt;
                    margin-left: 15px;
                    margin-right: 15px;
                }
                table {
                    font-size: 12pt;
                    font-family: Helvetica, Verdana, Geneva, Arial, sans-serif;
                    font-family: arial, sans-serif;
                    border-collapse: collapse;
                    align: center;
                }
                td {
                    text-align: left;
                    padding: 8px;
                }
                th {
                    text-align: left;
                    padding: 8px;
                    background-color: #ffffff;
                    border-bottom: 1px solid #333333;
                    text-align: center;
                }
                tr:nth-child(1) {
                    border-bottom: 1px solid #333333;
                    border-top: 2px solid #333333;
                }
                tr:last-child {
                    border-bottom: 2px solid #333333;
                }
                tr:nth-child(even) {
                    background-color: #dddddd;
                }
                .numberCell {
                    text-align: right;
                }
                .headerCell {
                    text-align: center;
                }
            </style>
        </head>
        <body>";
        f.write(s.as_bytes()).unwrap();
        s = "<body><h1>Kappa Index of Agreement</h1>";
        f.write(s.as_bytes()).unwrap();
        let s2 = &format!("{}{}{}{}{}", "<p><b>Input Data:</b> <br><br><b>Classification Data:</b> ", input_file1, "<br><br><b>Reference Data:</b> ", input_file2, "<p>");
        f.write(s2.as_bytes()).unwrap();
        s = "<br><table>";
        f.write(s.as_bytes()).unwrap();
        s = "<caption>Contingency Table</caption>";
        f.write(s.as_bytes()).unwrap();
        s = "<tr>";
        f.write(s.as_bytes()).unwrap();
        let s3 = &format!("{}{}{}", "<th colspan=\"2\" rowspan=\"2\"></th><th colspan=\"", num_classes, "\">Reference Data</th><th rowspan=\"2\">Row<br>Totals</th>");
        f.write(s3.as_bytes()).unwrap();
        s = "</tr>";
        f.write(s.as_bytes()).unwrap();
        s = "<tr>";
        f.write(s.as_bytes()).unwrap();
        for a in 0..256 {
            if active_class[a] {
                let s = &format!("{}{}{}", "<th>", convert_class_val_to_class_string(a as u8), "</th>");
                f.write(s.as_bytes()).unwrap();
            }
        }

        s = "</tr>";
        f.write(s.as_bytes()).unwrap();
        let mut first_entry = true;
        for a in 0..256 {
            if active_class[a] {
                if first_entry {
                    let s = format!("{}{}{}{}{}", "<tr><td rowspan=\"", num_classes, "\" valign=\"center\"><b>Class<br>Data</b></td> <td><b>", convert_class_val_to_class_string(a as u8), "</b></td>");
                    f.write(s.as_bytes()).unwrap();
                } else {
                    let s = format!("{}{}{}", "<tr><td><b>", convert_class_val_to_class_string(a as u8), "</b></td>");
                    f.write(s.as_bytes()).unwrap();
                }
                row_total = 0;
                for b in 0..256 {
                    if active_class[b] {
                        row_total += error_matrix[a][b];
                        let s = format!("{}{}{}", "<td>", error_matrix[a][b], "</td>");
                        f.write(s.as_bytes()).unwrap();
                    }
                }
                let s = format!("{}{}{}", "<td>", row_total, "</td>");
                f.write(s.as_bytes()).unwrap();

                let s2 = "</tr>";
                f.write(s2.as_bytes()).unwrap();
                first_entry = false;
            }
        }
        s = "<tr>";
        f.write(s.as_bytes()).unwrap();
        s = "<th colspan=\"2\">Column Totals</th>";
        f.write(s.as_bytes()).unwrap();
        for a in 0..256 {
            if active_class[a] {
                col_total = 0;
                for b in 0..256 {
                    if active_class[b] {
                        col_total += error_matrix[b][a];
                    }
                }
                let s = &format!("{}{}{}", "<td>", col_total, "</td>");
                f.write(s.as_bytes()).unwrap();
            }
        }

        let s4 = &format!("{}{}{}", "<td><b>N</b>=", n, "</td></tr>");
        f.write(s4.as_bytes()).unwrap();
        s = "</table>";
        f.write(s.as_bytes()).unwrap();
        s = "<br><br><table>";
        f.write(s.as_bytes()).unwrap();
        s = "<caption>Class Statistics</caption>";
        f.write(s.as_bytes()).unwrap();
        s = "<tr><th class=\"headerCell\">Class</th><th class=\"headerCell\">User's Accuracy<sup>1</sup><br>(Reliability)</th><th class=\"headerCell\">Producer's Accuracy<sup>1</sup><br>(Accuracy)</th></tr>";
        f.write(s.as_bytes()).unwrap();

        let mut average_producers = 0.0;
        let mut average_users = 0.0;
        let mut num_active = 0.0;
        for a in 0..256 {
            if active_class[a] {
                num_active += 1.0;
                let mut row_total = 0;
                let mut col_total = 0;
                for b in 0..256 {
                    if active_class[b] {
                        col_total += error_matrix[a][b];
                        row_total += error_matrix[b][a];
                    }
                }
                average_users += 100.0 * error_matrix[a][a] as f64 / col_total as f64;
                average_producers += 100.0 * error_matrix[a][a] as f64 / row_total as f64;
                let s = &format!("{}{}{}{}{}{}{}", "<tr><td>",  convert_class_val_to_class_string(a as u8), "</td><td class=\"numberCell\">", format!("{:.*}", 2, (100.0 * error_matrix[a][a] as f64 / col_total as f64)),
                        "%</td><td class=\"numberCell\">", format!("{:.*}", 2, (100.0 * error_matrix[a][a] as f64 / row_total as f64)), "%</td></tr>");
                f.write(s.as_bytes()).unwrap();
            }
        }
        f.write(format!("<tr><td>Average</td><td class=\"numberCell\">{}%</td><td class=\"numberCell\">{}%</td></tr>", format!("{:.*}", 2, average_users / num_active),
                format!("{:.*}", 2, average_producers / num_active)).as_bytes()).unwrap();


        s = "</table>";
        f.write(s.as_bytes()).unwrap();
        let s6 = &format!("<p>{}{}</p>", "<p><b>Overall Accuracy</b> = ", format!("{:.*}%", 2, overall_accuracy * 100.0));
        f.write(s6.as_bytes()).unwrap();
        let s7 = &format!("<p><b>Kappa</b><sup>2</sup> = {}</p>", format!("{:.*}", 3, kappa));
        f.write(s7.as_bytes()).unwrap();
        let s5 = &format!("{}{}", "<p><br>Notes:<br>1. User's accuracy refers to the proportion of points correctly assigned to a class (i.e. the number of points correctly classified for a category divided by the row total in the contingency table) and is a measure of the reliability. ",
                "Producer's accuracy is a measure of the proportion of the points in each category correctly classified (i.e. the number of points correctly classified for a category divided by the column total in the contingency table) and is a measure of the accuracy.<br>");
        f.write(s5.as_bytes()).unwrap();
        f.write("<br>2. Cohen's kappa coefficient is a statistic that measures inter-rater agreement for qualitative (categorical)
        items. It is generally thought to be a more robust measure than simple percent agreement calculation, since
        kappa takes into account the agreement occurring by chance. Kappa measures the percentage of data values in the
        main diagonal of the table and then adjusts these values for the amount of agreement that could be expected due
        to chance alone.</p>".as_bytes()).unwrap();
        s = "</body>";
        f.write_all(s.as_bytes())?;

        let _ = f.flush();

        if verbose {
            if cfg!(target_os = "macos") || cfg!(target_os = "ios") {
                let output = Command::new("open")
                    .arg(output_file.clone())
                    .output()
                    .expect("failed to execute process");

                let _ = output.stdout;
            } else if cfg!(target_os = "windows") {
                // let output = Command::new("cmd /c start")
                let output = Command::new("explorer.exe")
                    .arg(output_file.clone())
                    .output()
                    .expect("failed to execute process");

                let _ = output.stdout;
            } else if cfg!(target_os = "linux") {
                let output = Command::new("xdg-open")
                    .arg(output_file.clone())
                    .output()
                    .expect("failed to execute process");

                let _ = output.stdout;
            }

            println!("Complete! Please see {} for output.", output_file);
        }

        let end = time::now();
        let elapsed_time = end - start;

        println!("\n{}", &format!("Elapsed Time (excluding I/O): {}", elapsed_time).replace("PT", ""));

        Ok(())
    }
}
