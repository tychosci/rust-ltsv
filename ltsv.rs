/*!
Library for reading/writing Labeled Tab-Separated Values

# Example

~~~~~~~~~~~~~~~~~~~~~~
extern mod ltsv;
use ltsv::LTSVWriter;
use ltsv::LTSVReader;

fn main() {
    let infile = io::file_reader(&Path("path/fo/file.tlsv")).get();
    for infile.read_ltsv().each |record| {
        for record.each |&(label, value)| {
            io::println(fmt!("label: %s, value: %s", *label, *value));
        }
    }
}
~~~~~~~~~~~~~~~~~~~~~~
*/
#[link(name = "ltsv",
       vers = "0.1",
       uuid = "E0EA0251-E165-4612-919F-38E89ACECBE9",
       url  = "https://github.com/tychosci/rust-ltsv/")];

#[comment = "Library for reading/writing Labeled Tab-Separated Values"];
#[license = "MIT license"];
#[crate_type = "lib"];

use core::container::Map;
use core::hashmap::linear::LinearMap;
use io::WriterUtil;

pub type Record = LinearMap<~str, ~str>;

enum Trailing { EOF, NL, TAB }

enum ParseType {
    Record,
    Field,
    FieldLabel,
    FieldValue(Trailing)
}

enum ParseResult<T> {
    ParseError(~str),
    ParseOk(ParseType, T)
}

pub trait LTSVWriter {
    fn write_ltsv(&self, ltsv: &[Record]);
    fn write_ltsv_record(&self, record: &Record);
}

pub trait LTSVReader {
    fn read_ltsv(&self) -> ~[Record];
    fn read_ltsv_record(&self) -> Record;
}

pub impl<T: io::Writer> LTSVWriter for T {
    fn write_ltsv(&self, ltsv: &[Record]) {
        for ltsv.each |record| {
            self.write_ltsv_record(record);
            self.write_char('\n');
        }
    }
    fn write_ltsv_record(&self, record: &Record) {
        let mut is_first = true;
        for record.each |&(k, v)| {
            if !is_first { self.write_char('\t'); }
            self.write_str(fmt!("%s:%s", *k, *v));
            if is_first { is_first = false; }
        }
    }
}

pub impl<T: io::Reader> LTSVReader for T {
    fn read_ltsv(&self) -> ~[Record] {
        match parse_records(self) {
            ParseOk(_, maps) => maps,
            ParseError(reason) => die!(reason)
        }
    }
    fn read_ltsv_record(&self) -> Record {
        match parse_fields(self) {
            ParseOk(_, map) => map,
            ParseError(reason) => die!(reason)
        }
    }
}

fn parse_records<T: io::Reader>(rd: &T) -> ParseResult<~[Record]> {
    let mut maps = ~[];
    loop {
        match parse_fields(rd) {
            ParseOk(_, map) => {
                maps.push(map);
                if rd.eof() { break; }
            }
            ParseError(reason) => {
                return ParseError(reason);
            }
        }
    }
    ParseOk(Record, maps)
}

fn parse_fields<T: io::Reader>(rd: &T) -> ParseResult<Record> {
    let mut linear_map = LinearMap::new();
    loop {
        let label = match parse_field_label(rd) {
            ParseOk(_, label)  => label,
            ParseError(reason) => return ParseError(reason)
        };
        match parse_field_value(rd) {
            ParseOk(FieldValue(TAB), value) => {
                linear_map.insert(label, value);
            }
            ParseOk(_, value) => {
                linear_map.insert(label, value);
                return ParseOk(Field, linear_map);
            }
            ParseError(reason) => {
                return ParseError(reason);
            }
        }
    }
}

fn parse_field_label<T: io::Reader>(rd: &T) -> ParseResult<~str> {
    let mut bytes = ~[];
    loop {
        let b = rd.read_byte();
        match b {
            0x30..0x39 | 0x41..0x5a | 0x61..0x7a | 0x5f |
            0x2e | 0x2d => bytes.push(b as u8),
            0x3a if bytes.len() == 0 => return ParseError(~"label is empty"),
            0x3a => return ParseOk(FieldLabel, str::from_bytes(bytes)),
            -1   => return ParseError(~"EOF while parsing field label"),
            _    => return ParseError(~"invalid byte detected")
        }
    }
}

fn parse_field_value<T: io::Reader>(rd: &T) -> ParseResult<~str> {
    let mut bytes = ~[];
    loop {
        let b = rd.read_byte();
        match b {
            0x01..0x08 | 0x0b | 0x0c |
            0x0e..0xff => bytes.push(b as u8),
            0x0d => return try_read_trailing_LF(rd, str::from_bytes(bytes)),
            0x0a => return ParseOk(FieldValue(NL), str::from_bytes(bytes)),
            0x09 => return ParseOk(FieldValue(TAB), str::from_bytes(bytes)),
            -1   => return ParseOk(FieldValue(EOF), str::from_bytes(bytes)),
            _    => return ParseError(~"invalid byte detected")
        }
    }
}

#[inline(always)]
fn try_read_trailing_LF<T: io::Reader>(rd: &T, rv: ~str) -> ParseResult<~str> {
    if rd.read_byte() != 0x0a {
        ParseError(~"CR detected, but not provided with LF")
    } else {
        ParseOk(FieldValue(NL), rv)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::io::WriterUtil;

    #[test]
    fn test_parse_simple() {
        let ms = io::with_str_reader(~"a:1\tb:2", |rd| rd.read_ltsv());
        assert ms.len() == 1;
    }

    #[test]
    fn test_parse_full() {
        let s = io::with_str_writer(|wr| {
            // genzairyou
            wr.write_str(fmt!("%s:%s\t", "tofu", "豆"));
            wr.write_str(fmt!("%s:%s\t", "kamaboko", "魚"));
            wr.write_str(fmt!("%s:%s\n", "sukonbu", "海藻"));
            // konomi
            wr.write_str(fmt!("%s:%s\t", "tofu", "好き"));
            wr.write_str(fmt!("%s:%s\t", "kamaboko", "普通"));
            wr.write_str(fmt!("%s:%s\n", "sukonbu", "苦手"));
        });
        let ms = io::with_str_reader(s, |rd| rd.read_ltsv());
        assert ms.len() == 2;
    }

    #[test]
    fn test_parse_ltsv_trailing_nl_and_write() {
        let s = io::with_str_writer(|wr| {
            wr.write_str(fmt!("%s:%s\t", "neko", "yes"));
            wr.write_str(fmt!("%s:%s\t", "inu", "yes"));
            wr.write_str(fmt!("%s:%s\n", "tori", "yes"));
        });
        let ltsv = io::with_str_reader(s, |rd| rd.read_ltsv());
        let s2 = io::with_str_writer(|wr| wr.write_ltsv(ltsv));
        let ltsv2 = io::with_str_reader(s2, |rd| rd.read_ltsv());
        assert ltsv == ltsv2;
    }
}
