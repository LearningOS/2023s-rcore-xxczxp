
use alloc::collections::BTreeMap;
use alloc::vec::{Vec};

pub struct LockGuard {
    pub record:BTreeMap<usize,SourceRecord>
}

pub struct SourceRecord {
    pub available :usize,
    /// save the thread's allocation to this lock
    pub allocation:BTreeMap<usize,usize>,
    pub need:BTreeMap<usize,usize>,
}

fn add_one_create(map:&mut BTreeMap<usize,usize>,id:usize){
    if let Some(t)=map.get_mut(&id) {
        *t+=1;
    }
    else {
        map.insert(id, 1);
    }
}

/// id must in it
fn sub_one(map:&mut BTreeMap<usize,usize>,id:usize){
    let t=map.get_mut(&id).unwrap();
    assert!(*t != 0 as usize);
    *t-=1;
}


impl LockGuard {
    pub fn add_source(&mut self,s_id:usize,avai_num:usize){
        self.record.insert(s_id,SourceRecord::new(avai_num));
    }

    pub fn delete_source(&mut self,s_id:usize,avai_num:usize){
        self.record.insert(s_id,SourceRecord::new(avai_num));
    }

    /// need-1 available-1 allocation+1
    /// must call need_source_one first
    fn get_source_one(&mut self,tid:usize,s_id:usize) {
        let source_record: &mut SourceRecord=self.record.get_mut(&s_id).unwrap();
        assert!(source_record.available!=0);
        source_record.available-=1;
        add_one_create(&mut source_record.allocation,tid);
        sub_one(&mut source_record.need,tid);
    }

    /// need+1
    pub fn need_source_one(&mut self,tid:usize,s_id:usize) {
        let source_record=self.record.get_mut(&s_id).unwrap();
        add_one_create(&mut source_record.need,tid);
    }

    /// available+1 allocation-1
    pub fn return_source_one(&mut self,tid:usize,s_id:usize) {
        let source_record=self.record.get_mut(&s_id).unwrap();
        sub_one(&mut source_record.allocation,tid);
        source_record.available+=1;
    }

    pub fn check_deadlock(&self,tids:&[usize]) -> bool{
        let mut finish: Vec<bool>=Vec::new();
        let mut work:Vec<usize>=Vec::new();
        let mut allocation:Vec<Vec<usize>>=Vec::new();
        let mut need:Vec<Vec<usize>>=Vec::new();
        for (s_id,source) in self.record.iter(){
            work.push(source.available);
            finish.push(false);
            let mut i_allocation:Vec<usize>=Vec::new();
            let mut i_need:Vec<usize>=Vec::new();
            for tid in tids {
                i_allocation.push(source.allocation.get(tid).unwrap().clone());
                i_need.push(source.need.get(tid).unwrap().clone());
            }
            allocation.push(i_allocation);
            need.push(i_need);
        }
        'outer: loop {
            for i in 0..tids.len() {
                if finish[i] == false {
                    if need.iter().enumerate().all(|(j,s)| s[i] == work[j]){
                        finish[i] = true;
                        let _v=allocation.iter().enumerate().map(|(j,s)| {work[j]+=s[i]}).collect::<()>();
                    }
                }
                continue 'outer;
            }
            return finish.iter().all(|&x| x);
        }
        

    }

}

impl SourceRecord {
    fn new(avai:usize)-> Self{
        Self{
            available:avai,
            allocation:BTreeMap::new(),
            need: BTreeMap::new()
        }
    }

}