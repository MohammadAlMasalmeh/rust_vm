pub struct AddressLayout {
    pub offset_bits: u32,
    pub level_bits: Vec<u32>, 
}

impl AddressLayout {
    pub fn split(&self, addr: u64) -> Option<(Vec<usize>, u64)> {
        assert!(!self.level_bits.is_empty(), "At least one level is required");
        let offset = addr & ((1 << self.offset_bits) - 1);
        let mut rest = addr >> self.offset_bits;
        let mut indices = Vec::new();
        // Iterate backwards over the size of each level and get the index
        for &width in self.level_bits.iter().rev(){
            indices.push((rest & ((1 << width) - 1)) as usize);
            rest >>= width;
        }
        if rest != 0 {
            return None;
        }
        indices.reverse();
        Some((indices, offset))
    }
}

pub struct Pte{
    pub frame: usize,
    pub valid: bool,
    pub reference: bool,
    pub dirty: bool,
}

pub enum Entry{
    Empty, 
    Page(Pte),
    Table(Node)
}

pub struct Node{
    pub entries: Vec<Entry>,
}

impl Node{
    pub fn new(width: u32) -> Self{
        let mut entries = Vec::new();
        for _ in 0..(1 << width){
            entries.push(Entry::Empty);
        }
        Node { entries }
    }
}

pub struct PageTable{
    pub root: Node,
    pub layout: AddressLayout,
}

impl PageTable{
    pub fn new(layout: AddressLayout) -> Self{
        let root = Node::new(layout.level_bits[0]);
        PageTable { root, layout }
    }

    pub fn lookup(&self, addr: u64) -> Option<&Pte>{
        let (indices, _) = self.layout.split(addr)?;
        let mut node = &self.root;
        for &i in &indices{
            match &node.entries[i]{
                Entry::Empty => return None,
                Entry::Page(pte) => return Some(pte),
                Entry::Table(next) => node = next,
            }
        }
        None
    }

    pub fn mutable_lookup(&mut self, addr: u64) -> Option<&mut Pte>{
        let (indices, _) = self.layout.split(addr)?;
        let mut node = &mut self.root;
        for&i in &indices{
            match &mut node.entries[i]{
                Entry::Empty => return None,
                Entry::Page(pte) => return Some(pte),
                Entry::Table(next) => node = next,
            }
        }
        None
    }

    pub fn map(&mut self, addr: u64, frame: usize){
        let (indices, _) = self.layout.split(addr).expect("address out of range");
        let mut node = &mut self.root;
        for level in 0..indices.len() - 1{
            let i = indices[level];
        
            if let Entry::Empty = &node.entries[i]{
                node.entries[i] = Entry::Table(Node::new(self.layout.level_bits[level + 1]));
            }
            node = match &mut node.entries[i] {
                Entry::Table(next) => next,
                _ => panic!("expected a table at level {}", level),
            };
        }
        node.entries[indices[indices.len() - 1]] = Entry::Page(Pte{frame, valid: true, reference: false, dirty: false});
    }

    pub fn unmap(&mut self, addr: u64) -> Option<(usize, bool)> {
        let (indices, _) = self.layout.split(addr)?;
        let mut node = &mut self.root;
    
        for &i in &indices {
            match &mut node.entries[i] {
                Entry::Empty => return None,
                Entry::Table(next) => node = next,
                Entry::Page(pte) => {
                    if !pte.valid {
                        return None;
                    }
                    let dirty = pte.dirty;
                    pte.valid = false;
                    pte.reference = false;
                    pte.dirty = false;
                    return Some((pte.frame, dirty));
                }
            }
        }
        None
    }

}

#[cfg(test)]
mod tests {
    use super::*;

    fn x86() -> AddressLayout {
        AddressLayout { offset_bits: 12, level_bits: vec![9, 9, 9, 9] }
    }

    #[test]
    fn splits_basic_addresses() {
        assert_eq!(x86().split(0x0), Some((vec![0, 0, 0, 0], 0)));
        assert_eq!(x86().split(0x1000), Some((vec![0, 0, 0, 1], 0)));
        assert_eq!(x86().split(0x1abc), Some((vec![0, 0, 0, 1], 0xabc)));
    }

    #[test]
    fn splits_each_level() {
        assert_eq!(x86().split(1 << 21), Some((vec![0, 0, 1, 0], 0)));
        assert_eq!(x86().split(1 << 30), Some((vec![0, 1, 0, 0], 0)));
        assert_eq!(x86().split(1 << 39), Some((vec![1, 0, 0, 0], 0)));
        assert_eq!(
            x86().split((1 << 39) | (2 << 30) | (3 << 21) | (4 << 12) | 0x5ab),
            Some((vec![1, 2, 3, 4], 0x5ab))
        );
    }

    #[test]
    fn works_with_other_layouts() {
        let l = AddressLayout { offset_bits: 12, level_bits: vec![7, 8] };
        assert_eq!(l.split(1 << 20), Some((vec![1, 0], 0)));
        assert_eq!(l.split((3 << 20) | (5 << 12) | 0x42), Some((vec![3, 5], 0x42)));
    }

    #[test]
    fn rejects_out_of_range() {
        assert_eq!(x86().split(1 << 48), None);
        let l = AddressLayout { offset_bits: 12, level_bits: vec![7, 8] };
        assert_eq!(l.split(1 << 27), None);
    }

    #[test]
    #[should_panic]
    fn panics_with_no_levels() {
        let l = AddressLayout { offset_bits: 12, level_bits: vec![] };
        l.split(0);
    }

    #[test]
    fn new_node_is_all_empty() {
        let n = Node::new(9);
        assert_eq!(n.entries.len(), 512);
        assert!(n.entries.iter().all(|e| matches!(e, Entry::Empty)));
    }

    #[test]
    fn lookup_on_empty_table() {
        let pt = PageTable::new(x86());
        assert!(pt.lookup(0x1000).is_none());
        assert!(pt.lookup(1 << 48).is_none());
    }

    #[test]
    fn map_then_lookup() {
        let mut pt = PageTable::new(x86());
        pt.map(0x1000, 7);
        let pte = pt.lookup(0x1abc).unwrap();
        assert_eq!(pte.frame, 7);
        assert!(pte.valid);
        assert!(!pte.reference);
        assert!(!pte.dirty);
        assert!(pt.lookup(0x2000).is_none());
    }

    #[test]
    fn only_creates_needed_branches() {
        let mut pt = PageTable::new(x86());
        pt.map(0x1000, 7);
        assert!(matches!(pt.root.entries[0], Entry::Table(_)));
        assert!(matches!(pt.root.entries[1], Entry::Empty));
    }

    #[test]
    fn pages_share_branches() {
        let mut pt = PageTable::new(x86());
        pt.map(0x1000, 7);
        pt.map(0x2000, 8);
        assert_eq!(pt.lookup(0x1000).unwrap().frame, 7);
        assert_eq!(pt.lookup(0x2000).unwrap().frame, 8);
    }

    #[test]
    fn far_apart_pages_get_separate_branches() {
        let mut pt = PageTable::new(x86());
        pt.map(0x1000, 7);
        pt.map(1 << 39, 9);
        assert!(matches!(pt.root.entries[0], Entry::Table(_)));
        assert!(matches!(pt.root.entries[1], Entry::Table(_)));
        assert_eq!(pt.lookup(1 << 39).unwrap().frame, 9);
    }

    #[test]
    fn map_works_with_other_layouts() {
        let l = AddressLayout { offset_bits: 12, level_bits: vec![7, 8] };
        let mut pt = PageTable::new(l);
        pt.map((3 << 20) | (5 << 12), 42);
        assert_eq!(pt.lookup((3 << 20) | (5 << 12) | 0x10).unwrap().frame, 42);
    }

    #[test]
    #[should_panic]
    fn map_out_of_range_panics() {
        let mut pt = PageTable::new(x86());
        pt.map(1 << 48, 7);
    }

    #[test]
    fn unmap_marks_not_valid() {
        let mut pt = PageTable::new(x86());
        pt.map(0x1000, 7);
        assert_eq!(pt.unmap(0x1000), Some((7, false)));
        assert!(!pt.lookup(0x1000).unwrap().valid);
        assert_eq!(pt.unmap(0x1000), None);
    }

    #[test]
    fn unmap_missing_page() {
        let mut pt = PageTable::new(x86());
        assert_eq!(pt.unmap(0x1000), None);
        assert_eq!(pt.unmap(1 << 48), None);
    }

    #[test]
    fn remap_after_unmap() {
        let mut pt = PageTable::new(x86());
        pt.map(0x1000, 7);
        pt.unmap(0x1000);
        pt.map(0x1000, 3);
        let pte = pt.lookup(0x1000).unwrap();
        assert_eq!(pte.frame, 3);
        assert!(pte.valid);
    }

    #[test]
    fn unmap_returns_dirty() {
        let mut pt = PageTable::new(x86());
        pt.map(0x1000, 7);
        pt.mutable_lookup(0x1000).unwrap().dirty = true;
        assert_eq!(pt.unmap(0x1000), Some((7, true)));
    }
}