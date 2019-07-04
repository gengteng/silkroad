use futures::{Stream, Async, Future};
use std::path::PathBuf;
use walkdir::{DirEntry, IntoIter, WalkDir};

pub struct WalkStream {
    iter: IntoIter
}

impl WalkStream {
    pub fn new<P: Into<PathBuf>>(root: P) -> Self {
        WalkStream {
            iter: walkdir::WalkDir::new(root.into()).into_iter()
        }
    }
}

impl From<WalkDir> for WalkStream {
    fn from(wd: WalkDir) -> Self {
        WalkStream {
            iter: wd.into_iter()
        }
    }
}

impl Stream for WalkStream {
    type Item = DirEntry;
    type Error = walkdir::Error;

    fn poll(&mut self) -> Result<Async<Option<Self::Item>>, Self::Error> {
        let next = self.iter.next();
        match next {
            Some(result) => {
                match result {
                    Ok(entry) => {
                        Ok(Async::Ready(Some(entry)))
                    }
                    Err(e) => {
                        Err(e)
                    }
                }
            }
            None => Ok(Async::Ready(None))
        }
    }
}

pub struct DownloadCrates {
    registry: Registry
}

impl Stream for DownloadCrates {
    type Item = ();
    type Error = ();

    fn poll(&mut self) -> Result<Async<Option<Self::Item>>, Self::Error> {
        unimplemented!()
    }
}