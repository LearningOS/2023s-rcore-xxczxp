//! index node(inode, namely file control block) layer
//!
//! The data struct and functions for the inode layer that service file-related system calls
//!
//! NOTICE: The difference between [`Inode`] and [`DiskInode`]  can be seen from their names: DiskInode in a relatively fixed location within the disk block, while Inode Is a data structure placed in memory that records file inode information.
use crate::BLOCK_SZ;

use super::{
    block_cache_sync_all, get_block_cache, BlockDevice, DirEntry, DiskInode, DiskInodeType,
    EasyFileSystem, DIRENT_SZ,
};
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;
use log::error;
use spin::{Mutex, MutexGuard};

/// Inode struct in memory
pub struct Inode {
    /// The block id of the inode
    block_id: usize,
    /// The offset of the inode in the block
    block_offset: usize,
    /// The file system
    fs: Arc<Mutex<EasyFileSystem>>,
    /// The block device
    block_device: Arc<dyn BlockDevice>,
}

impl Inode {
    /// Create a new Disk Inode
    ///
    /// We should not acquire efs lock here.
    pub fn new(
        block_id: u32,
        block_offset: usize,
        fs: Arc<Mutex<EasyFileSystem>>,
        block_device: Arc<dyn BlockDevice>,
    ) -> Self {
        Self {
            block_id: block_id as usize,
            block_offset,
            fs,
            block_device,
        }
    }
    /// read the content of the disk inode on disk with 'f' function
    fn read_disk_inode<V>(&self, f: impl FnOnce(&DiskInode) -> V) -> V {
        get_block_cache(self.block_id, Arc::clone(&self.block_device))
            .lock()
            .read(self.block_offset, f)
    }
    /// modify the content of the disk inode on disk with 'f' function
    fn modify_disk_inode<V>(&self, f: impl FnOnce(&mut DiskInode) -> V) -> V {
        get_block_cache(self.block_id, Arc::clone(&self.block_device))
            .lock()
            .modify(self.block_offset, f)
    }
    /// find the disk inode id according to the file with 'name' by search the directory entries in the disk inode with Directory type
    fn find_inode_id(&self, name: &str, disk_inode: &DiskInode) -> Option<u32> {
        // assert it is a directory
        assert!(disk_inode.is_dir());
        let file_count = (disk_inode.size as usize) / DIRENT_SZ;
        let mut dirent = DirEntry::empty();
        for i in 0..file_count {
            assert_eq!(
                disk_inode.read_at(DIRENT_SZ * i, dirent.as_bytes_mut(), &self.block_device,),
                DIRENT_SZ,
            );
            if dirent.name() == name {
                return Some(dirent.inode_id() as u32);
            }
        }
        None
    }
    /// find the disk inode of the file with 'name'

    fn find_entry_id(&self, name: &str, disk_inode: &DiskInode) -> Option<u32> {
        // assert it is a directory
        assert!(disk_inode.is_dir());
        let file_count = (disk_inode.size as usize) / DIRENT_SZ;
        let mut dirent = DirEntry::empty();
        for i in 0..file_count {
            assert_eq!(
                disk_inode.read_at(DIRENT_SZ * i, dirent.as_bytes_mut(), &self.block_device,),
                DIRENT_SZ,
            );
            if dirent.name() == name {
                return Some(i as u32);
            }
        }
        None
    }

    /// Find inode under current inode by name
    pub fn find(&self, name: &str) -> Option<Arc<Inode>> {
        let fs = self.fs.lock();
        self.read_disk_inode(|disk_inode| {
            self.find_inode_id(name, disk_inode).map(|inode_id| {
                let (block_id, block_offset) = fs.get_disk_inode_pos(inode_id);
                Arc::new(Self::new(
                    block_id,
                    block_offset,
                    self.fs.clone(),
                    self.block_device.clone(),
                ))
            })
        })
    }
    /// increase the size of file( also known as 'disk inode')
    fn increase_size(
        &self,
        new_size: u32,
        disk_inode: &mut DiskInode,
        fs: &mut MutexGuard<EasyFileSystem>,
    ) {
        if new_size < disk_inode.size {
            return;
        }
        let blocks_needed = disk_inode.blocks_num_needed(new_size);
        let mut v: Vec<u32> = Vec::new();
        for _ in 0..blocks_needed {
            v.push(fs.alloc_data());
        }
        disk_inode.increase_size(new_size, v, &self.block_device);
    }
    /// create a file with 'name' in the root directory
    pub fn create(&self, name: &str) -> Option<Arc<Inode>> {
        let mut fs = self.fs.lock();
        let op = |root_inode: &mut DiskInode| {
            // assert it is a directory
            assert!(root_inode.is_dir());
            // has the file been created?
            self.find_inode_id(name, root_inode)
        };
        if self.modify_disk_inode(op).is_some() {
            return None;
        }
        // create a new file
        // alloc a inode with an indirect block
        let new_inode_id = fs.alloc_inode();
        // initialize inode
        let (new_inode_block_id, new_inode_block_offset) = fs.get_disk_inode_pos(new_inode_id);
        get_block_cache(new_inode_block_id as usize, Arc::clone(&self.block_device))
            .lock()
            .modify(new_inode_block_offset, |new_inode: &mut DiskInode| {
                new_inode.initialize(DiskInodeType::File);
            });
        self.modify_disk_inode(|root_inode| {
            // append file in the dirent
            let file_count = (root_inode.size as usize) / DIRENT_SZ;
            let new_size = (file_count + 1) * DIRENT_SZ;
            // increase size
            self.increase_size(new_size as u32, root_inode, &mut fs);
            // write dirent
            let dirent = DirEntry::new(name, new_inode_id);
            root_inode.write_at(
                file_count * DIRENT_SZ,
                dirent.as_bytes(),
                &self.block_device,
            );
        });

        let (block_id, block_offset) = fs.get_disk_inode_pos(new_inode_id);
        block_cache_sync_all();
        // return inode
        Some(Arc::new(Self::new(
            block_id,
            block_offset,
            self.fs.clone(),
            self.block_device.clone(),
        )))
        // release efs lock automatically by compiler
    }
    /// create a directory with 'name' in the root directory
    ///
    /// list the file names in the root directory
    pub fn ls(&self) -> Vec<String> {
        let _fs = self.fs.lock();
        self.read_disk_inode(|disk_inode| {
            let file_count = (disk_inode.size as usize) / DIRENT_SZ;
            let mut v: Vec<String> = Vec::new();
            for i in 0..file_count {
                let mut dirent = DirEntry::empty();
                assert_eq!(
                    disk_inode.read_at(i * DIRENT_SZ, dirent.as_bytes_mut(), &self.block_device,),
                    DIRENT_SZ,
                );
                v.push(String::from(dirent.name()));
            }
            v
        })
    }
    /// Read the content in offset position of the file into 'buf'
    pub fn read_at(&self, offset: usize, buf: &mut [u8]) -> usize {
        let _fs = self.fs.lock();
        self.read_disk_inode(|disk_inode| disk_inode.read_at(offset, buf, &self.block_device))
    }
    /// Write the content in 'buf' into offset position of the file
    pub fn write_at(&self, offset: usize, buf: &[u8]) -> usize {
        let mut fs = self.fs.lock();
        let size = self.modify_disk_inode(|disk_inode| {
            self.increase_size((offset + buf.len()) as u32, disk_inode, &mut fs);
            disk_inode.write_at(offset, buf, &self.block_device)
        });
        block_cache_sync_all();
        size
    }
    /// Set the file(disk inode) length to zero, delloc all data blocks of the file.
    pub fn clear(&self) {
        let mut fs = self.fs.lock();
        self.modify_disk_inode(|disk_inode| {
            let size = disk_inode.size;
            let data_blocks_dealloc = disk_inode.clear_size(&self.block_device);
            assert!(data_blocks_dealloc.len() == DiskInode::total_blocks(size) as usize);
            for data_block in data_blocks_dealloc.into_iter() {
                fs.dealloc_data(data_block);
            }
        });
        block_cache_sync_all();
    }

    /// delete an entry from inode
    /// demand inode's entry are all file
    pub fn delete_entry(&self, name:&str)-> isize {
        let mut fs: MutexGuard<EasyFileSystem> = self.fs.lock();
        self.modify_disk_inode(|disk_inode| {
            let entry_id = match self.find_entry_id(name,&disk_inode) {
                Some(a)=>a as usize,
                None=>{
                    error!("not find entry");
                    return -1;
                },

            };
            assert!(entry_id < ((disk_inode.size as usize) / DIRENT_SZ));
            let mut dirent = DirEntry::empty();
            disk_inode.read_at(
                entry_id * DIRENT_SZ,
                dirent.as_bytes_mut(),
                &self.block_device,
            );

            // delete entry
            let file_count = (disk_inode.size as usize) / DIRENT_SZ;
            let mut last_dirent = DirEntry::empty();
            disk_inode.read_at(
                (file_count - 1) * DIRENT_SZ,
                last_dirent.as_bytes_mut(),
                &self.block_device,
            );
            // move last entry to deleted entry
            disk_inode.write_at(
                entry_id * DIRENT_SZ,
                last_dirent.as_bytes_mut(),
                &self.block_device,
            );
            // set zero for the last slot
            disk_inode.write_at(
                (file_count - 1) * DIRENT_SZ,
                DirEntry::empty().as_bytes_mut(),
                &self.block_device,
            );
            // resize, delete blocks if needed
            let blocks_dealloc =
                disk_inode.decrease_size(disk_inode.size - DIRENT_SZ as u32, &self.block_device);
            for data_block in blocks_dealloc.into_iter() {
                fs.dealloc_data(data_block);
            }

            let (target_blocks, target_offset) = fs.get_disk_inode_pos(dirent.inode_id());
            let mut need_to_dealloc = false;
            get_block_cache(target_blocks as usize, Arc::clone(&(self.block_device)))
                .lock()
                .modify(target_offset, |target_inode: &mut DiskInode| {
                    target_inode.strong_count -= 1;
                    if target_inode.strong_count == 0 {
                        if target_inode.is_dir() {
                            todo!();
                        }
                        need_to_dealloc = true;
                    }
                });
            if need_to_dealloc {
                fs.dealloc_inode(dirent.inode_id());
            }
            return 0;
        })
    }

    /// get current inode id
    pub fn get_inode_id(&self) -> usize {
        let inode_size = core::mem::size_of::<DiskInode>();
        let fs: MutexGuard<EasyFileSystem> = self.fs.lock();
        let node_offset =
            (self.block_id - fs.inode_area_start_block as usize) * BLOCK_SZ + self.block_offset;
        node_offset / inode_size
    }

    /// get current node's strong_count
    pub fn get_strong_count(&self) -> usize {
        let _fs: MutexGuard<EasyFileSystem> = self.fs.lock();
        self.read_disk_inode(|disk_inode| disk_inode.strong_count as usize)
    }

    /// is dir?
    pub fn is_dir(&self) -> bool {
        let _fs: MutexGuard<EasyFileSystem> = self.fs.lock();
        self.read_disk_inode(|disk_inode| disk_inode.is_dir())
    }

    /// is file?
    pub fn is_file(&self) -> bool {
        let _fs: MutexGuard<EasyFileSystem> = self.fs.lock();
        self.read_disk_inode(|disk_inode| disk_inode.is_file())
    }
    /// lab6 for linkat
    pub fn create_from_inode(&self, name: &str, other: Arc<Inode>) -> Option<Arc<Inode>> {
        // get the inode id that is going to be linked
        let linked_inode_id = other.get_inode_id() as u32;

        let mut fs = self.fs.lock();
        let op = |root_inode: &DiskInode| {
            // assert it is a directory
            assert!(root_inode.is_dir());
            // has the file been created?
            self.find_inode_id(name, root_inode)
        };
        if self.read_disk_inode(op).is_some() {
            return None;
        }

        // increase strong count
        other.modify_disk_inode(|root_inode| {
            root_inode.strong_count+=1;
        });

        // add entry
        self.modify_disk_inode(|root_inode| {
            // append file in the dirent
            let file_count = (root_inode.size as usize) / DIRENT_SZ;
            let new_size = (file_count + 1) * DIRENT_SZ;
            // increase size
            self.increase_size(new_size as u32, root_inode, &mut fs);
            // write dirent
            let dirent = DirEntry::new(name, linked_inode_id);
            root_inode.write_at(
                file_count * DIRENT_SZ,
                dirent.as_bytes(),
                &self.block_device,
            );
        });
        block_cache_sync_all();
        // return the gaven inode
        Some(other)
        // release efs lock automatically by compiler
    }
}
