extern crate filetime;

use std::collections::HashMap;
use std::fs;
use std::fs::File;
use std::path::Path;
use std::io::{ BufRead, BufReader };
use std::{ thread, time };
use filetime::FileTime;
use std::default::Default;

pub struct HotloadedFile<RowType: Default> {
    file_path: &'static str,
    section_header: &'static str,
    parsing_function: fn(&String, &String, &mut RowType),

    last_loaded_time: FileTime,
    contents: HashMap<String, RowType>,
}


impl<RowType: Default> HotloadedFile<RowType> {
    pub fn new(file_path: &'static str, section_header: &'static str, parsing_function: fn(&String, &String, &mut RowType)) -> HotloadedFile<RowType> {
        HotloadedFile {
            file_path: file_path,
            section_header: section_header,
            parsing_function: parsing_function,

            last_loaded_time: FileTime::zero(),
            contents: HashMap::new(),
        }
    }

    /// Checks the last write date of the file to determine whether the file should be reloaded.
    /// If the file date is more recent than the last read date, the file is reparsed.
    pub fn hotload(&mut self) -> Result<bool, String> {
        let path = Path::new(self.file_path); 
        if !path.exists() {
            return Err(format!("File {:?} does not exist", self.file_path));
        }

        let metadata = path.metadata().unwrap();
        match metadata.modified() {
            Err(e) => Err(format!("Couldn't get hotloaded file metadata: {:?}", e)),
            Ok(time) => {
                let modification_time = FileTime::from_last_modification_time(&metadata);
                if modification_time > self.last_loaded_time {
                    let result = self.reload_file();
                    match result {
                        Ok(_) => {
                            self.last_loaded_time = modification_time;
                            result
                        },
                        Err(e) => { Err(e) }
                    }
                } else {
                    // The file was not modified since last being loaded. Nothing to do.
                    Ok(false) 
                }
            },
        }
    }

    fn reload_file(&mut self) -> Result<bool, String> {
        self.contents.clear();

        // Keep track of the current header for loading the parsed lines into the contents
        let mut current_header: String = "".to_string();
        let mut section_content: RowType = Default::default();

        let mut file = File::open(self.file_path).unwrap();
        let mut buffer = BufReader::new(&file);
        for line in buffer.lines() {
            let l = line.unwrap();
            if l.is_empty() {
                continue;
            }

            if l.starts_with(self.section_header) {
                if !current_header.is_empty() {
                    self.contents.insert(current_header.clone(), section_content);
                    section_content = Default::default();
                }
                current_header = (&l[self.section_header.len()..]).to_string();
                continue;
            }

            if current_header.is_empty() {
                return Err("Cannot have values under no section".to_string());
            }

            (self.parsing_function)(&current_header, &l, &mut section_content);
        }

        if !current_header.is_empty() {
            self.contents.insert(current_header.clone(), section_content);
            section_content = Default::default();
        }

        Ok(true)
    }
}

/// Simple parsing function for testing. Real implementation should be provided by user.
fn my_parse(section_header: &String, row: &String, current: &mut u32) { 
    match row.parse::<u32>() {
        Ok(num) => { *current = num; },
        Err(_) => { }
    };
}

fn main() {
    let mut hotloaded_file = HotloadedFile::new("assets/hotloadedfile.txt", ":", my_parse);

    let half_second = time::Duration::from_millis(500);

    loop {
        match hotloaded_file.hotload() {
            Ok(true) => println!("File reloaded: {:?}", hotloaded_file.contents),
            Err(e) => println!("{:?}", e),
            _ => {},
        };

        thread::sleep(half_second);
        
        //assert!(hotloaded_file.contents.contains_key("Section1"));
        // assert!(hotloaded_file.contents.contains_key("Section2"));
    }
}
