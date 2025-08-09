use std::path::PathBuf;

use clap::ArgMatches;

pub(crate) fn package(_project_path: PathBuf, _sub_matches: &ArgMatches) {
    todo!()
}

pub(crate) fn build(_project_path: PathBuf, _sub_matches: &ArgMatches) {
    todo!()
}

pub(crate) fn health() {
    todo!()
}

#[allow(dead_code)]
pub(crate) fn play(_project_path: &PathBuf) {}
