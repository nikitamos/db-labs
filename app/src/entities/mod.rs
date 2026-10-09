pub mod user;

#[derive(Debug)]
pub struct PageRequest<Oft: Ord + Default = i32> {
    pub limit: i64,
    pub offset: Oft,
}

impl<Oft: Ord + Default> PageRequest<Oft> {
    pub fn next_from(self, new_offset: Oft) -> Self {
        Self {
            limit: self.limit,
            offset:new_offset
        }
    }
}

pub struct Page<T, Oft: Ord + Default = i32> {
    pub items: Vec<T>,
    pub next_request: PageRequest<Oft>,
}

impl<T, Oft: Ord + Default> Page<T, Oft> {
    pub fn empty(limit: i64) -> Self {
        Self {
            items: vec![],
            next_request: PageRequest {
                limit,
                offset: Default::default(),
            },
        }
    }
}
