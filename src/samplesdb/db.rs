use rusqlite::{Connection, NO_PARAMS};
use std::collections::hash_map::Values;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use crate::samplesdb::error::SamplesDBError;

#[derive(Debug)]
pub struct Sample<'a> {
  pub id: i64,
  pub path: String,
  pub tags: Vec<&'a Tag>,
}

#[derive(Debug)]
pub struct Tag {
  pub id: i64,
  pub name: String,
}

pub struct SamplesDB<'a> {
  samples: HashMap<i64, Sample<'a>>,
  tags: HashMap<i64, Tag>,
  pub base_path: PathBuf,

  connection: Connection,
}

const SUPPORTED_AUDIO_FILES: [&str; 6] = ["aiff", "flac", "midi", "ogg", "wav", "mp3"];

impl SamplesDB<'_> {
  pub fn open(db_path: &Path, base_path: &Path) -> Result<Self, SamplesDBError> {
    let mut db = Self {
      samples: HashMap::new(),
      tags: HashMap::new(),
      base_path: base_path.to_owned(),
      connection: Connection::open(db_path)?,
    };

    db.setup_tables()?;
    db.load_tags()?;
    db.load_samples()?;

    Ok(db)
  }

  fn setup_tables(&self) -> Result<(), SamplesDBError> {
    self.connection.execute(
      "CREATE TABLE IF NOT EXISTS sample (
                id   INTEGER PRIMARY KEY,
                path TEXT NOT NULL UNIQUE 
            )",
      NO_PARAMS,
    )?;

    self.connection.execute(
      "CREATE TABLE IF NOT EXISTS tag (
                id   INT PRIMARY KEY,
                name TEXT NOT NULL UNIQUE 
            )",
      NO_PARAMS,
    )?;

    self.connection.execute(
      "CREATE TABLE IF NOT EXISTS sample_tag (
                sample_id INT NOT NULL,
                tag_id    INT NOT NULL,
                UNIQUE(sample_id, tag_id) ON CONFLICT REPLACE
            )",
      NO_PARAMS,
    )?;

    Ok(())
  }

  fn load_tags(&mut self) -> Result<(), SamplesDBError> {
    let mut stmt = self.connection.prepare("SELECT id, name FROM tag;")?;

    let tags: Result<Vec<Tag>, _> = stmt
      .query_map(NO_PARAMS, |row| {
        Ok(Tag {
          id: row.get(0)?,
          name: row.get(1)?,
        })
      })?
      .collect();

    for tag in tags? {
      self.tags.insert(tag.id, tag);
    }

    Ok(())
  }

  fn load_samples(&mut self) -> Result<(), SamplesDBError> {
    for entry in WalkDir::new(&self.base_path) {
      let path_str = entry?.path().to_path_buf();

      if let Some(extension) = path_str.extension() {
        if SUPPORTED_AUDIO_FILES.iter().any(|&ext| ext == extension) {
          self.add_sample(
            (&path_str)
              .strip_prefix(&self.base_path)
              .unwrap()
              .to_str()
              .unwrap(),
          )?;
        }
      }
    }

    Ok(())
  }

  fn add_sample<'a>(&mut self, path: &str) -> Result<(), SamplesDBError> {
    let result = self.connection.query_row(
      "SELECT id FROM sample WHERE path = ?1;",
      params![path],
      |row| row.get(0),
    );

    let id: i64 = result.or_else(|_| -> Result<i64, SamplesDBError> {
      self
        .connection
        .execute("INSERT INTO sample (path) VALUES (?1);", params![path])?;
      Ok(self.connection.last_insert_rowid())
    })?;

    let sample = Sample {
      id,
      path: path.to_string(),
      tags: Vec::new(),
    };

    self.samples.insert(sample.id, sample);

    Ok(())
  }

  fn create_tag<P: Into<String> + Copy + rusqlite::ToSql>(
    &mut self,
    name: P,
  ) -> Result<i64, SamplesDBError> {
    self
      .connection
      .execute("INSERT INTO tag (name) VALUES (?1);", params![&name])?;

    let tag = Tag {
      id: self.connection.last_insert_rowid(),
      name: name.into().clone(),
    };

    let opt = self.tags.insert(tag.id, tag);

    Ok(opt.as_ref().unwrap().id)
  }

  pub fn samples(&self) -> Values<i64, Sample> {
    self.samples.values()
  }

  pub fn sample_id_by_path(&self, path: &str) -> Option<i64> {
    for sample in self.samples.values() {
      if sample.path == path {
        return Some(sample.id);
      }
    }

    None
  }

  pub fn full_path_of_sample(&self, sample_id: i64) -> PathBuf {
    let mut path = self.base_path.clone();
    path.push(&self.samples[&sample_id].path);
    path
  }
}
