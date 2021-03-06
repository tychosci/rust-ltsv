rust-ltsv
=========

Library for reading/writing [LTSV](http://ltsv.org/) data from Rust.

Example
-------

reading file with LTSV format:

    extern mod ltsv;
    use ltsv::LTSVReader;

    fn main() {
        let infile = io::file_reader(&Path("path/fo/file.tlsv")).get();
        for infile.each_ltsv_record |record| {
            for record.each |&(label, value)| {
                io::println(fmt!("%s: %s", *label, *value));
            }
        }
    }

writing LTSV data:

    extern mod ltsv;
    use core::container::Map;
    use core::hashmap::linear::LinearMap;
    use ltsv::LTSVWriter;

    fn main() {
        let mut records = ~[];
        let mut record = LinearMap::new();

        record.insert(~"host", ~"$remote_addr");
        record.insert(~"user", ~"$remote_user");
        record.insert(~"status", ~"$status");
        records.push(record);

        let ltsv_str = do io::with_str_writer |wr| {
            wr.write_ltsv(records);
        };
        assert fmt!("%s:%s\t%s:%s\t%s:%s\n",
                    "host", "$remote_addr",
                    "user", "$remote_user",
                    "status", "$status") == ltsv_str;
    }

LICENSE
-------

(The MIT License)

See the LICENSE file for details.
