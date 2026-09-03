//
// Copyright 2023, Colias Group, LLC
//
// SPDX-License-Identifier: BSD-2-Clause
//

use super::scheme::{Level, Scheme};
use super::table::{AbstractEntry, Table};

impl Table {
    pub(crate) fn embed(&self, scheme: &Scheme, vaddr: u64, thead_maee: bool) -> (Vec<u8>, u64) {
        Embedding::new(scheme, vaddr, thead_maee).embed(self)
    }
}

struct Embedding<'a> {
    scheme: &'a Scheme,
    start_vaddr: u64,
    buf: Vec<u8>,
    thead_maee: bool,
}

impl<'a> Embedding<'a> {
    fn new(scheme: &'a Scheme, start_vaddr: u64, thead_maee: bool) -> Self {
        Self {
            scheme,
            start_vaddr,
            buf: vec![],
            thead_maee,
        }
    }

    fn embed(mut self, table: &Table) -> (Vec<u8>, u64) {
        let root_vaddr = self.embed_inner(table, 0);
        (self.buf, root_vaddr)
    }

    fn embed_inner(&mut self, table: &Table, level: Level) -> u64 {
        let bytes = table
            .entries
            .iter()
            .flat_map(|entry| {
                self.scheme.descriptor_to_bytes(match entry {
                    AbstractEntry::Empty => self.scheme.empty_descriptor(),
                    AbstractEntry::Leaf(descriptor) => *descriptor,
                    AbstractEntry::Branch(branch) => {
                        let child_vaddr = self.embed_inner(branch, level + 1);
                        let descriptor = self.scheme.branch_descriptor(child_vaddr);
                        if self.thead_maee
                            && matches!(self.scheme, Scheme::RiscVSv39 | Scheme::RiscVSv32)
                        {
                            descriptor | (0b0_1110_u64 << 59)
                        } else {
                            descriptor
                        }
                    }
                })
            })
            .collect::<Vec<_>>();
        self.align(1 << self.scheme.level_align_bits(level));
        let vaddr = self.cur_vaddr();
        self.buf.extend(bytes);
        vaddr
    }

    fn cur_vaddr(&self) -> u64 {
        self.start_vaddr + u64::try_from(self.buf.len()).unwrap()
    }

    fn align(&mut self, align: u64) {
        let cur_vaddr = self.cur_vaddr();
        let aligned_vaddr = cur_vaddr.next_multiple_of(align);
        self.buf
            .resize((aligned_vaddr - self.start_vaddr).try_into().unwrap(), 0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const THEAD_PMA: u64 = 0b0_1110_u64 << 59;

    #[test]
    fn thead_maee_marks_riscv_branch_descriptors() {
        let table = Table {
            entries: vec![AbstractEntry::Branch(Box::new(Table {
                entries: vec![AbstractEntry::Empty],
            }))],
        };
        let scheme = Scheme::RiscVSv39;
        let (plain, plain_root) = table.embed(&scheme, 0x8000_0000, false);
        let (marked, marked_root) = table.embed(&scheme, 0x8000_0000, true);
        let plain_offset = usize::try_from(plain_root - 0x8000_0000).unwrap();
        let marked_offset = usize::try_from(marked_root - 0x8000_0000).unwrap();
        let plain_branch =
            u64::from_le_bytes(plain[plain_offset..plain_offset + 8].try_into().unwrap());
        let marked_branch =
            u64::from_le_bytes(marked[marked_offset..marked_offset + 8].try_into().unwrap());

        assert_eq!(plain_branch & THEAD_PMA, 0);
        assert_eq!(marked_branch & THEAD_PMA, THEAD_PMA);
        assert_eq!(plain_branch, marked_branch & !THEAD_PMA);
    }
}
