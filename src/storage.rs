use lmdb::{
    Cursor, Database, DatabaseFlags, Environment, EnvironmentFlags, Transaction, WriteFlags,
};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TreeMetadata {
    pub num_leaves: usize,
    pub max_leaves: usize,
}

#[derive(Debug)]
pub struct LmdbStorage {
    env: Environment,
    leaves_db: Database,
    cache_db: Database,
    metadata_db: Database,
}

impl LmdbStorage {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn std::error::Error>> {
        let env = Environment::new()
            .set_flags(EnvironmentFlags::NO_SUB_DIR)
            .set_max_dbs(4)
            .set_map_size(1024 * 1024 * 1024) // 1GB
            .open(path.as_ref())?;

        let leaves_db = env.create_db(Some("leaves"), DatabaseFlags::empty())?;
        let cache_db = env.create_db(Some("cache"), DatabaseFlags::empty())?;
        let metadata_db = env.create_db(Some("metadata"), DatabaseFlags::empty())?;

        Ok(Self {
            env,
            leaves_db,
            cache_db,
            metadata_db,
        })
    }

    // Leaf operations
    pub fn store_leaf(&self, index: usize, leaf: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
        let mut txn = self.env.begin_rw_txn()?;
        let key = index.to_be_bytes();
        txn.put(self.leaves_db, &key, &leaf, WriteFlags::empty())?;
        txn.commit()?;
        Ok(())
    }

    pub fn get_leaf(&self, index: usize) -> Result<Option<Vec<u8>>, Box<dyn std::error::Error>> {
        let txn = self.env.begin_ro_txn()?;
        let key = index.to_be_bytes();
        match txn.get(self.leaves_db, &key) {
            Ok(data) => Ok(Some(data.to_vec())),
            Err(lmdb::Error::NotFound) => Ok(None),
            Err(e) => Err(Box::new(e)),
        }
    }

    pub fn get_all_leaves(&self) -> Result<Vec<Vec<u8>>, Box<dyn std::error::Error>> {
        let txn = self.env.begin_ro_txn()?;
        let mut cursor = txn.open_ro_cursor(self.leaves_db)?;
        let mut leaves = Vec::new();

        for (_key, value) in cursor.iter() {
            leaves.push(value.to_vec());
        }

        Ok(leaves)
    }

    pub fn store_leaves_batch(&self, leaves: &[Vec<u8>]) -> Result<(), Box<dyn std::error::Error>> {
        let mut txn = self.env.begin_rw_txn()?;

        for (index, leaf) in leaves.iter().enumerate() {
            let key = index.to_be_bytes();
            txn.put(self.leaves_db, &key, &leaf, WriteFlags::empty())?;
        }

        txn.commit()?;
        Ok(())
    }

    pub fn append_leaves(
        &self,
        start_index: usize,
        leaves: &[Vec<u8>],
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut txn = self.env.begin_rw_txn()?;

        for (offset, leaf) in leaves.iter().enumerate() {
            let key = (start_index + offset).to_be_bytes();
            txn.put(self.leaves_db, &key, &leaf, WriteFlags::empty())?;
        }

        txn.commit()?;
        Ok(())
    }

    // Cache operations
    pub fn store_cache_level(
        &self,
        level: usize,
        hashes: &[Vec<u8>],
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut txn = self.env.begin_rw_txn()?;
        let key = format!("level_{}", level);
        let serialized = bincode::serialize(hashes)?;
        txn.put(self.cache_db, &key, &serialized, WriteFlags::empty())?;
        txn.commit()?;
        Ok(())
    }

    pub fn get_cache_level(
        &self,
        level: usize,
    ) -> Result<Option<Vec<Vec<u8>>>, Box<dyn std::error::Error>> {
        let txn = self.env.begin_ro_txn()?;
        let key = format!("level_{}", level);
        match txn.get(self.cache_db, &key) {
            Ok(data) => {
                let hashes: Vec<Vec<u8>> = bincode::deserialize(data)?;
                Ok(Some(hashes))
            }
            Err(lmdb::Error::NotFound) => Ok(None),
            Err(e) => Err(Box::new(e)),
        }
    }

    pub fn get_all_cache_levels(&self) -> Result<Vec<Vec<Vec<u8>>>, Box<dyn std::error::Error>> {
        let txn = self.env.begin_ro_txn()?;
        let mut cursor = txn.open_ro_cursor(self.cache_db)?;
        let mut levels = Vec::new();

        for (_key, value) in cursor.iter() {
            let hashes: Vec<Vec<u8>> = bincode::deserialize(value)?;
            levels.push(hashes);
        }

        Ok(levels)
    }

    pub fn store_cache_batch(
        &self,
        cache_levels: &[Vec<Vec<u8>>],
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut txn = self.env.begin_rw_txn()?;

        for (level, hashes) in cache_levels.iter().enumerate() {
            let key = format!("level_{}", level);
            let serialized = bincode::serialize(hashes)?;
            txn.put(self.cache_db, &key, &serialized, WriteFlags::empty())?;
        }

        txn.commit()?;
        Ok(())
    }

    pub fn clear_cache(&self) -> Result<(), Box<dyn std::error::Error>> {
        let mut txn = self.env.begin_rw_txn()?;
        txn.clear_db(self.cache_db)?;
        txn.commit()?;
        Ok(())
    }

    // Metadata operations
    pub fn store_metadata(
        &self,
        metadata: &TreeMetadata,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut txn = self.env.begin_rw_txn()?;
        let serialized = bincode::serialize(metadata)?;
        txn.put(
            self.metadata_db,
            &"tree_metadata",
            &serialized,
            WriteFlags::empty(),
        )?;
        txn.commit()?;
        Ok(())
    }

    pub fn get_metadata(&self) -> Result<Option<TreeMetadata>, Box<dyn std::error::Error>> {
        let txn = self.env.begin_ro_txn()?;
        match txn.get(self.metadata_db, &"tree_metadata") {
            Ok(data) => {
                let metadata: TreeMetadata = bincode::deserialize(data)?;
                Ok(Some(metadata))
            }
            Err(lmdb::Error::NotFound) => Ok(None),
            Err(e) => Err(Box::new(e)),
        }
    }

    pub fn store_root(&self, root: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
        let mut txn = self.env.begin_rw_txn()?;
        txn.put(self.metadata_db, &"cached_root", &root, WriteFlags::empty())?;
        txn.commit()?;
        Ok(())
    }

    pub fn get_root(&self) -> Result<Option<Vec<u8>>, Box<dyn std::error::Error>> {
        let txn = self.env.begin_ro_txn()?;
        match txn.get(self.metadata_db, &"cached_root") {
            Ok(data) => Ok(Some(data.to_vec())),
            Err(lmdb::Error::NotFound) => Ok(None),
            Err(e) => Err(Box::new(e)),
        }
    }

    // Utility operations
    pub fn clear_all(&self) -> Result<(), Box<dyn std::error::Error>> {
        let mut txn = self.env.begin_rw_txn()?;
        txn.clear_db(self.leaves_db)?;
        txn.clear_db(self.cache_db)?;
        txn.clear_db(self.metadata_db)?;
        txn.commit()?;
        Ok(())
    }

    pub fn sync(&self) -> Result<(), Box<dyn std::error::Error>> {
        self.env.sync(true)?;
        Ok(())
    }

    pub fn store_node(
        &self,
        level: usize,
        index: u64,
        hash: &[u8],
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut txn = self.env.begin_rw_txn()?;
        let key = format!("{:02}:{:016x}", level, index);
        txn.put(self.cache_db, &key, &hash, WriteFlags::empty())?;
        txn.commit()?;
        Ok(())
    }

    pub fn get_node(
        &self,
        level: usize,
        index: u64,
    ) -> Result<Option<Vec<u8>>, Box<dyn std::error::Error>> {
        let txn = self.env.begin_ro_txn()?;
        let key = format!("{:02}:{:016x}", level, index);
        match txn.get(self.cache_db, &key) {
            Ok(data) => Ok(Some(data.to_vec())),
            Err(lmdb::Error::NotFound) => Ok(None),
            Err(e) => Err(Box::new(e)),
        }
    }
}
