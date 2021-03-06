// Copyright 2016 PingCAP, Inc.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// See the License for the specific language governing permissions and
// limitations under the License.

use util::worker::Runnable;
use util::rocksdb;
use util::escape;

use rocksdb::DB;
use std::sync::Arc;
use std::fmt::{self, Formatter, Display};
use std::error;
use super::metrics::COMPACT_RANGE_CF;

pub struct Task {
    pub cf_name: String,
    pub start_key: Option<Vec<u8>>, // None means smallest key
    pub end_key: Option<Vec<u8>>, // None means largest key
}

impl Display for Task {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f,
               "Compact CF[{}], range[{:?}, {:?}]",
               self.cf_name,
               self.start_key.as_ref().map(|k| escape(&k)),
               self.end_key.as_ref().map(|k| escape(&k)))
    }
}

quick_error! {
    #[derive(Debug)]
    enum Error {
        Other(err: Box<error::Error + Sync + Send>) {
            from()
            cause(err.as_ref())
            description(err.description())
            display("compact failed {:?}", err)
        }
    }
}

pub struct Runner {
    engine: Arc<DB>,
}

impl Runner {
    pub fn new(engine: Arc<DB>) -> Runner {
        Runner { engine: engine }
    }

    fn compact_range_cf(&mut self,
                        cf_name: String,
                        start_key: Option<Vec<u8>>,
                        end_key: Option<Vec<u8>>)
                        -> Result<(), Error> {
        let cf_handle = box_try!(rocksdb::get_cf_handle(&self.engine, &cf_name));
        let compact_range_timer = COMPACT_RANGE_CF.with_label_values(&[&cf_name])
            .start_timer();
        self.engine.compact_range_cf(cf_handle,
                                     start_key.as_ref().map(Vec::as_slice),
                                     end_key.as_ref().map(Vec::as_slice));

        compact_range_timer.observe_duration();
        Ok(())
    }
}

impl Runnable<Task> for Runner {
    fn run(&mut self, task: Task) {
        let cf = task.cf_name.clone();
        if let Err(e) = self.compact_range_cf(task.cf_name, task.start_key, task.end_key) {
            error!("execute compact range for cf {} failed, err {}", &cf, e);
        } else {
            info!("compact range for cf {} finished", &cf);
        }
    }
}
