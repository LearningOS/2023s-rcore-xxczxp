use core::cmp::Ordering;
use crate::config::{BIGSTRDE};

use super::manager::get_min_stride;

#[derive(Clone,Copy)]
pub struct Stride(pub u64);

impl PartialOrd for Stride {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        if self.0 == other.0 {
            return None;
        }
        let mut is_less=self.0 < other.0;
        let differ= if self.0 > other.0 {self.0 - other.0}else{other.0 - self.0};
        if  differ > BIGSTRDE/2 {
            is_less = !is_less;
        }
        if is_less {
            Some(Ordering::Less)
        }else {
            Some(Ordering::Greater)
        }
    }
}



impl Eq for Stride {

}

impl Ord for Stride {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering { 
        if self.0 == other.0 {
            return Ordering::Equal;
        }
        let mut is_less=self.0 < other.0;
        let differ= if self.0 > other.0 {self.0 - other.0}else{other.0 - self.0};
        if  differ > BIGSTRDE/2 {
            is_less = !is_less;
        }
        if is_less {
            Ordering::Less
        }else {
            Ordering::Greater
        }
    }
}

#[allow(unused)]
impl PartialEq for Stride {
    fn eq(&self, other: &Self) -> bool {
        false
    }
}


impl Stride {
    pub fn init() -> Self {
        get_min_stride()
    }
    
}