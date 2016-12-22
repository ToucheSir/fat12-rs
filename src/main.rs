extern crate byteorder;
extern crate chrono;

use std::env;
use std::io::prelude::*;
use std::io::SeekFrom;
use std::fs::File;
use byteorder::{LittleEndian, ByteOrder};
use chrono::*;

const OS_NAME: usize = 3;
const OS_NAME_SIZE: usize = 8;
const BYTES_PER_SECTOR: usize = 11;
const SECTORS_PER_CLUSTER: usize = 13;
const RESERVED_SECTORS: usize = 14;
const FATS: usize = 16;
const ROOT_DIR_ENTRIES: usize = 17;
const TOTAL_SECTORS: usize = 19;
const SECTORS_PER_FAT: usize = 22;
const SECTORS_PER_TRACK: usize = 24;
const HEADS: usize = 26;
const FAT32_TOTAL_SECTORS: usize = 32;
const BOOT_SIGNATURE: usize = 38;
const VOLUME_ID: usize = 39;
const VOLUME_LABEL: usize = 43;
const VOLUME_LABEL_SIZE: usize = 11;
const FS_TYPE: usize = 54;
const FS_TYPE_SIZE: usize = 54;

struct DiskInfo {
    os_name: [u8; 8],
    bytes_per_sector: u16,
    sectors_per_cluster: u8,
    reserved_sectors: u16,
    fats: u8,
    root_dir_entries: u16,
    total_sectors: u16,
    sectors_per_fat: u16,
    sectors_per_track: u16,
    heads: u16,
    boot_signature: u8,
    volume_id: u32,
    volume_label: [u8; 11],
    fs_type: [u8; 54],
}
impl DiskInfo {
    fn new(buf: &[u8]) -> Self {
        DiskInfo {
            os_name: {
                let mut name = [0; OS_NAME_SIZE];
                name.copy_from_slice(&buf[OS_NAME..OS_NAME + OS_NAME_SIZE]);
                name
            },
            bytes_per_sector: LittleEndian::read_u16(&buf[BYTES_PER_SECTOR..]),
            sectors_per_cluster: buf[SECTORS_PER_CLUSTER],
            reserved_sectors: LittleEndian::read_u16(&buf[RESERVED_SECTORS..]),
            fats: buf[FATS],
            root_dir_entries: LittleEndian::read_u16(&buf[ROOT_DIR_ENTRIES..]),
            total_sectors: LittleEndian::read_u16(&buf[TOTAL_SECTORS..]),
            sectors_per_fat: LittleEndian::read_u16(&buf[SECTORS_PER_FAT..]),
            sectors_per_track: LittleEndian::read_u16(&buf[SECTORS_PER_TRACK..]),
            heads: LittleEndian::read_u16(&buf[HEADS..]),
            boot_signature: buf[BOOT_SIGNATURE],
            volume_id: LittleEndian::read_u32(&buf[VOLUME_ID..]),
            volume_label: {
                let mut label = [0; VOLUME_LABEL_SIZE];
                label.copy_from_slice(&buf[VOLUME_LABEL..VOLUME_LABEL + VOLUME_LABEL_SIZE]);
                label
            },
            fs_type: {
                let mut ft = [0; FS_TYPE_SIZE];
                ft.copy_from_slice(&buf[FS_TYPE..FS_TYPE + FS_TYPE_SIZE]);
                ft
            },
        }
    }
}

fn read_disk_info(disk_file: &mut File) -> Result<DiskInfo, std::io::Error> {
    let mut buf = [0u8; 512];
    try!(disk_file.read_exact(&mut buf));
    Ok(DiskInfo::new(&buf))
}

const DIR_ENTRY_SIZE: usize = 32;
const DIR_ENTRY_NAME_SIZE: usize = 8;
const DIR_ENTRY_EXT: usize = 8;
const DIR_ENTRY_EXT_SIZE: usize = 3;
const DIR_ENTRY_ATTRS: usize = 11;
const DIR_ENTRY_RESERVED: usize = 12;
const DIR_ENTRY_CREATETIME: usize = 14;
const DIR_ENTRY_CREATEDATE: usize = 16;
const DIR_ENTRY_LASTACCESS: usize = 18;
const DIR_ENTRY_WRITETIME: usize = 22;
const DIR_ENTRY_WRITEDATE: usize = 24;
const DIR_ENTRY_FLC: usize = 26;
const DIR_ENTRY_FILESIZE: usize = 28;

enum DirEntryAttributes {
    ReadOnly = 0x01,
    Hidden = 0x02,
    System = 0x04,
    VolumeLabel = 0x08,
    SubDir = 0x10,
    Archive = 0x20,
}

struct DirEntry {
    file_name: [u8; DIR_ENTRY_NAME_SIZE],
    file_ext: [u8; DIR_ENTRY_EXT_SIZE],
    attributes: u8,
    reserved: u16,
    create_time: u16,
    create_date: u16,
    last_access_date: u16,
    last_write_time: u16,
    last_write_date: u16,
    flc: u16,
    file_size: u32,
}
impl DirEntry {
    fn new(buf: &[u8]) -> Self {
        DirEntry {
            file_name: {
                let mut name = [' ' as u8; DIR_ENTRY_NAME_SIZE];
                name.copy_from_slice(&buf[0..DIR_ENTRY_NAME_SIZE]);
                name
            },
            file_ext: {
                let mut ext = [' ' as u8; DIR_ENTRY_EXT_SIZE];
                ext.copy_from_slice(&buf[DIR_ENTRY_EXT..DIR_ENTRY_EXT + DIR_ENTRY_EXT_SIZE]);
                ext
            },
            attributes: buf[DIR_ENTRY_ATTRS],
            reserved: LittleEndian::read_u16(&buf[DIR_ENTRY_RESERVED..]),
            create_time: LittleEndian::read_u16(&buf[DIR_ENTRY_CREATETIME..]),
            create_date: LittleEndian::read_u16(&buf[DIR_ENTRY_CREATEDATE..]),
            last_access_date: LittleEndian::read_u16(&buf[DIR_ENTRY_LASTACCESS..]),
            last_write_time: LittleEndian::read_u16(&buf[DIR_ENTRY_WRITETIME..]),
            last_write_date: LittleEndian::read_u16(&buf[DIR_ENTRY_WRITETIME..]),
            flc: LittleEndian::read_u16(&buf[DIR_ENTRY_FLC..]),
            file_size: LittleEndian::read_u32(&buf[DIR_ENTRY_FILESIZE..]),
        }
    }
}

fn list_rootdir(info: &DiskInfo, disk_file: &mut File) -> Result<(), std::io::Error> {
    let root_dir_start =
        (info.bytes_per_sector * (info.fats as u16 * info.sectors_per_fat + 1)) as u64;
    disk_file.seek(SeekFrom::Start(root_dir_start))?;
    let mut entry_buf = [0; DIR_ENTRY_SIZE];
    for _ in 0..info.root_dir_entries {
        disk_file.read_exact(&mut entry_buf)?;
        if LittleEndian::read_u16(&entry_buf) & 0xFFF0 == 0 {
            break;
        }

        let entry = DirEntry::new(&entry_buf);
        if (entry.attributes & 0x0F) != 0 {
            continue;
        }
        let is_dir = (entry.attributes | DirEntryAttributes::SubDir as u8) == 0;
        print!("{} {} {}",
               if is_dir { 'd' } else { 'f' },
               entry.file_size,
               std::str::from_utf8(&entry.file_name).unwrap().trim());
        if !is_dir {
            print!(".{}", std::str::from_utf8(&entry.file_ext).unwrap().trim());
        }
        println!(" {}", to_datetime(entry.create_date, entry.create_time));
    }
    Ok(())
}

fn to_datetime(date: u16, time: u16) -> NaiveDateTime {
    NaiveDate::from_ymd((date >> 9) as i32 + 1980, (date & 0x01E0) as u32 >> 5, date as u32 & 0x001F)
        .and_hms(time as u32 >> 11, (time & 0x07E0) as u32 >> 5, (time & 0x001F) as u32)
}

fn main() {
    let mut args = env::args();
    if args.len() < 3 {
        println!("usage: fat12 command");
        return;
    }
    let (command, disk_path) = (args.nth(1).unwrap(), args.next().unwrap());
    let mut disk_file = File::open(disk_path).unwrap();

    match command.as_ref() {
        "info" => {
            let info = read_disk_info(&mut disk_file).unwrap();
            println!("{}", std::str::from_utf8(&info.os_name).unwrap());
            println!("0x{:X}", info.bytes_per_sector);
        }
        "list" => {
            let info = read_disk_info(&mut disk_file).unwrap();
            list_rootdir(&info, &mut disk_file).unwrap();
        }
        _ => (),
    }
}
