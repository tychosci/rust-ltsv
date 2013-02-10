rust-ltsv
=========

Library for Reading/Writing [LTSV](http://ltsv.org/) data from Rust.

Example
-------

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

LICENSE
-------

(The MIT License)

See the LICENSE file for details.
