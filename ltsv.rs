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
            io::println(fmt!("%s: %s", *label, *value));
        }
    }
}
~~~~~~~~~~~~~~~~~~~~~~
*/
#[link(name = "ltsv",
       vers = "0.2",
       uuid = "E0EA0251-E165-4612-919F-38E89ACECBE9",
       url  = "https://github.com/tychosci/rust-ltsv/")];

#[comment = "Library for reading/writing Labeled Tab-Separated Values"];
#[license = "MIT license"];
#[crate_type = "lib"];

use core::container::Map;
use core::hashmap::linear::LinearMap;
use core::io::WriterUtil;

pub type Record = LinearMap<~str, ~str>;

enum ParseType {
    FieldLabel,
    FieldValue,
    Field,
    Record,
    Ltsv
}

#[deriving_eq]
enum ParseDelimiter {
    EOF, TAB, NL, MISC
}

enum ParseResult<T> {
    ParseError(~str),
    ParseOk(ParseType, ParseDelimiter, T)
}

pub trait LTSVWriter {
    fn write_ltsv(&self, ltsv: &[Record]);
    fn write_ltsv_record(&self, record: &Record);
}

pub trait LTSVReader {
    fn read_ltsv(&self) -> ~[Record];
    fn each_ltsv_record(&self, f: &fn(&Record) -> bool);
    fn each_ltsv_field(&self, f: &fn(&(~str, ~str)) -> bool);
}

impl<T: io::Writer> LTSVWriter for T {
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

impl<T: io::Reader> LTSVReader for T {
    fn read_ltsv(&self) -> ~[Record] {
        let mut parser = LTSVParser::new(self);
        match parser.parse_ltsv() {
            ParseError(reason) => fail!(reason),
            ParseOk(_, _, records) => records
        }
    }
    fn each_ltsv_record(&self, f: &fn(&Record) -> bool) {
        let mut parser = LTSVParser::new(self);
        while !parser.eof() {
            match parser.parse_record() {
                ParseError(reason) => fail!(reason),
                ParseOk(_, _, record) => if !f(&record) { break; }
            }
        }
    }
    fn each_ltsv_field(&self, f: &fn(&(~str, ~str)) -> bool) {
        let mut parser = LTSVParser::new(self);
        while !parser.eof() {
            match parser.parse_field() {
                ParseError(reason) => fail!(reason),
                ParseOk(_, _, field) => if !f(&field) { break; }
            }
        }
    }
}

struct LTSVParser<T> {
    priv rd: &'self T,
    priv cur: @mut int
}

pub impl<T: io::Reader> LTSVParser<'self, T> {
    fn new(rd: &'r T) -> LTSVParser/&r<T> {
        let cur = @mut rd.read_byte();
        LTSVParser { rd: rd, cur: cur }
    }

    fn eof(&self) -> bool { *self.cur == -1 }

    fn bump(&self) {
        if !self.eof() {
            *self.cur = self.rd.read_byte();
        }
    }

    fn parse_ltsv(&self) -> ParseResult<~[Record]> {
        let mut records = ~[];
        loop {
            match self.parse_record() {
                ParseError(reason) => {
                    return ParseError(reason);
                }
                ParseOk(_, EOF, record) => {
                    records.push(record);
                    break;
                }
                ParseOk(_, _, record) => {
                    records.push(record);
                }
            }
        }
        ParseOk(Ltsv, EOF, records)
    }

    fn parse_record(&self) -> ParseResult<Record> {
        let mut record = LinearMap::new();
        loop {
            match self.parse_field() {
                ParseError(reason) => {
                    return ParseError(reason);
                }
                ParseOk(_, TAB, (label, value)) => {
                    record.insert(label, value);
                }
                ParseOk(_, delim, (label, value)) => {
                    record.insert(label, value);
                    return ParseOk(Record, delim, record);
                }
            }
        }
    }

    fn parse_field(&self) -> ParseResult<(~str, ~str)> {
        self.skip_whitespaces();
        let label = match self.parse_field_label() {
            ParseError(reason) => return ParseError(reason),
            ParseOk(_, _, label) => { self.bump(); label }
        };
        match self.parse_field_value() {
            ParseError(reason) => {
                ParseError(reason)
            }
            ParseOk(_, delim, value) => {
                self.bump();
                // avoid skipping whitespaces in the middle of parsing record.
                if delim != TAB { self.skip_whitespaces(); }
                // re-check EOF
                let delim = if self.eof() { EOF } else { delim };
                ParseOk(Field, delim, (label, value))
            }
        }
    }

    priv fn parse_field_label(&self) -> ParseResult<~str> {
        let mut bytes = ~[];
        loop {
            match *self.cur {
                0x30..0x39 | 0x41..0x5a | 0x61..0x7a | 0x5f |
                0x2e | 0x2d => bytes.push(*self.cur as u8),
                0x3a if bytes.len() == 0 => return ParseError(~"label is empty"),
                0x3a => return ParseOk(FieldLabel, MISC, str::from_bytes(bytes)),
                -1   => return ParseError(~"EOF while parsing field label"),
                _    => return ParseError(~"invalid byte detected")
            }
            self.bump();
        }
    }

    priv fn parse_field_value(&self) -> ParseResult<~str> {
        let mut bytes = ~[];
        loop {
            match *self.cur {
                0x01..0x08 | 0x0b | 0x0c |
                0x0e..0xff => bytes.push(*self.cur as u8),
                0x0d => return self.consume_forward_LF(str::from_bytes(bytes)),
                0x0a => return ParseOk(FieldValue, NL, str::from_bytes(bytes)),
                0x09 => return ParseOk(FieldValue, TAB, str::from_bytes(bytes)),
                -1   => return ParseOk(FieldValue, EOF, str::from_bytes(bytes)),
                _    => return ParseError(~"invalid byte detected")
            }
            self.bump();
        }
    }

    priv fn consume_forward_LF(&self, rv: ~str) -> ParseResult<~str> {
        self.bump();
        if *self.cur != 0x0a {
            ParseError(~"CR detected, but not provided with LF")
        } else {
            ParseOk(FieldValue, NL, rv)
        }
    }

    priv fn skip_whitespaces(&self) {
        while char::is_whitespace(*self.cur as char) {
            self.bump();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::io::WriterUtil;

    fn mk_record_string() -> ~str {
        do io::with_str_writer |wr| {
            // genzairyou
            wr.write_str(fmt!("%s:%s\t", "tofu", "豆"));
            wr.write_str(fmt!("%s:%s\t", "kamaboko", "魚"));
            wr.write_str(fmt!("%s:%s\n", "sukonbu", "海藻"));
            // konomi
            wr.write_str(fmt!("%s:%s\t", "tofu", "好き"));
            wr.write_str(fmt!("%s:%s\t", "kamaboko", "普通"));
            wr.write_str(fmt!("%s:%s\n", "sukonbu", "苦手"));
        }
    }

    #[test]
    fn test_parse_simple() {
        let records = io::with_str_reader(~"a:1\tb:2", |rd| rd.read_ltsv());
        fail_unless!(records.len() == 1);
    }

    #[test]
    fn test_parse_full() {
        let s = mk_record_string();
        let records = io::with_str_reader(s, |rd| rd.read_ltsv());
        fail_unless!(records.len() == 2);
    }

    #[test]
    fn test_parse_ltsv_trailing_nl_and_write() {
        let s = mk_record_string();
        let records_1 = io::with_str_reader(s, |rd| rd.read_ltsv());
        let s2 = io::with_str_writer(|wr| wr.write_ltsv(records_1));
        let records_2 = io::with_str_reader(s2, |rd| rd.read_ltsv());
        fail_unless!(records_1 == records_2);
    }

    #[test]
    fn test_each_read_each_record() {
        let s = mk_record_string();
        let ks = [~"tofu", ~"kamaboko", ~"sukonbu"];
        let vs = [~"豆", ~"魚", ~"海藻", ~"好き", ~"普通", ~"苦手"];
        do io::with_str_reader(s) |rd| {
            for rd.each_ltsv_record |record| {
                for record.each |&(k, v)| {
                    fail_unless!(ks.contains(k));
                    fail_unless!(vs.contains(v));
                }
            }
        }
    }
}
