use walkdir::WalkDir;
use std::{collections::HashMap, hash::Hasher, io, path::{Path, PathBuf}};
use std::fs::File;
use twox_hash::XxHash64;

struct HashWriter<T: Hasher>(T);

impl<T: Hasher> io::Write for HashWriter<T> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.write(buf);
        Ok(buf.len())
    }

    fn write_all(&mut self, buf: &[u8]) -> io::Result<()> {
        self.write(buf).map(|_| ())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn hash_file(file: &Path) -> u64{
    let mut f = File::open(file).expect("Unable to open file");
    let hasher = XxHash64::with_seed(0);
    let mut hw = HashWriter(hasher);
    io::copy(&mut f, &mut hw).expect("Unable to copy data");
    let hasher = hw.0;
    hasher.finish()
}


fn main() {

    let root = "/home/woelper";
    let mut db: HashMap<u64, Vec<PathBuf>> = HashMap::new();


    for entry in WalkDir::new(root).into_iter().filter_map(|e| e.ok()).filter(|e| e.path().is_file()) {
        let hash = hash_file(entry.path());
        let e = db.entry(hash).or_default();
        e.push(entry.path().to_owned());
    }
    
    //dbg!(db);
    // at this point we have all files ordered by hash, with filenames. Everything with more than one entry is a duplicate.

}