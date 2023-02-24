use std::{collections::HashMap, hash::Hash};

use itertools::Itertools as _;

use crate::LedgerEvent;

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct Block<BlockIdT, UserIdT, AmountT, PublicKeyT, SignatureT> {
    parent: Option<BlockIdT>,
    id: BlockIdT,
    events: Vec<LedgerEvent<UserIdT, AmountT, PublicKeyT, SignatureT>>,
}

/// Keeps track of blocks.
/// Does not perform any verification.
pub struct BlockGraph<BlockIdT, UserIdT, AmountT, PublicKeyT, SignatureT> {
    block_ids_to_blocks:
        HashMap<BlockIdT, Block<BlockIdT, UserIdT, AmountT, PublicKeyT, SignatureT>>,
    block_id_graph: petgraph::graphmap::DiGraphMap<BlockIdT, ()>,
    winning_chain: Vec<Block<BlockIdT, UserIdT, AmountT, PublicKeyT, SignatureT>>,
}

impl<BlockIdT, UserIdT, AmountT, PublicKeyT, SignatureT> Default
    for BlockGraph<BlockIdT, UserIdT, AmountT, PublicKeyT, SignatureT>
where
    BlockIdT: Copy + Ord + Hash, // TODO: PR with petgraph so that this isn't required
{
    fn default() -> Self {
        Self {
            block_ids_to_blocks: Default::default(),
            block_id_graph: petgraph::graphmap::DiGraphMap::new(),
            winning_chain: Default::default(),
        }
    }
}

impl<BlockIdT, UserIdT, AmountT, PublicKeyT, SignatureT>
    BlockGraph<BlockIdT, UserIdT, AmountT, PublicKeyT, SignatureT>
where
    BlockIdT: Hash + Eq + Clone + Ord + Copy,
    UserIdT: PartialEq + Clone,
    AmountT: PartialEq + Clone,
    PublicKeyT: PartialEq + Clone,
    SignatureT: PartialEq + Clone,
{
    pub fn add_block(
        &mut self,
        block: Block<BlockIdT, UserIdT, AmountT, PublicKeyT, SignatureT>,
    ) -> Result<(), AddBlockError> {
        use std::collections::hash_map::Entry;
        match self.block_ids_to_blocks.entry(block.id.clone()) {
            Entry::Occupied(already) if already.get() == &block => Ok(()), // idempotent
            Entry::Occupied(_) => Err(AddBlockError::WouldClobber),
            Entry::Vacant(vacancy) => {
                vacancy.insert(block.clone());
                match (block.parent, self.winning_chain.last()) {
                    (Some(parent), Some(tail)) if parent == tail.id => {
                        // fast path - we don't need to recalculate the winning chain
                        self.block_id_graph.add_edge(parent, block.id, ());
                        self.winning_chain.push(block);
                    }
                    (Some(parent), _) => {
                        self.block_id_graph.add_edge(parent, block.id, ());
                        self.winning_chain = self.calculate_winning_chain();
                    }
                    (None, _) => {
                        self.block_id_graph.add_node(block.id);
                        self.winning_chain = self.calculate_winning_chain();
                    }
                }
                Ok(())
            }
        }
    }

    pub fn winning_chain(&self) -> Vec<Block<BlockIdT, UserIdT, AmountT, PublicKeyT, SignatureT>> {
        self.winning_chain.clone()
    }

    fn calculate_winning_chain(
        &self,
    ) -> Vec<Block<BlockIdT, UserIdT, AmountT, PublicKeyT, SignatureT>> {
        let mut winner = Vec::new();
        for root in self.root_blocks() {
            for leaf in self.leaf_blocks() {
                if root == leaf && winner.len() == 0 {
                    winner.push(*root);
                    continue;
                }

                match petgraph::algo::all_simple_paths::<Vec<_>, _>(
                    &self.block_id_graph,
                    *root,
                    *leaf,
                    0,
                    None,
                )
                .at_most_one()
                {
                    Ok(Some(candidate)) if candidate.len() > winner.len() => winner = candidate,
                    Ok(Some(_)) | Ok(None) => (),
                    Err(_) => unreachable!("each block is unique, and has at most one parent, so there cannot be multiple paths between two blocks"),
                }
            }
        }
        winner
            .into_iter()
            .map(|block_id| {
                self.block_ids_to_blocks
                    .get(&block_id)
                    .expect("BlockGraph.blocks and BlockGraph.graph are out of sync")
                    .clone()
            })
            .collect()
    }

    fn root_blocks(&self) -> Vec<&BlockIdT> {
        self.block_ids_to_blocks
            .values()
            .filter_map(|it| match it.parent {
                Some(_) => None,
                None => Some(&it.id),
            })
            .sorted() // deterministic winning chain
            .collect()
    }
    fn leaf_blocks(&self) -> Vec<&BlockIdT> {
        self.block_ids_to_blocks
            .keys()
            .filter(|it| self.block_id_graph.neighbors(**it).count() == 0)
            .sorted() // deterministic winning chain
            .collect()
    }
}

#[derive(Debug, Clone, Copy, thiserror::Error)]
pub enum AddBlockError {
    #[error("block with the same id but different contents is already in the block graph")]
    WouldClobber,
}

#[cfg(test)]
mod tests {
    use super::*;

    type TestBlockGraph = BlockGraph<char, (), (), (), ()>;

    fn add_block(graph: &mut TestBlockGraph, parent: impl Into<Option<char>>, id: char) {
        graph
            .add_block(Block {
                parent: parent.into(),
                id,
                events: vec![],
            })
            .unwrap()
    }

    fn assert_winning_chain(graph: &TestBlockGraph, chain: impl IntoIterator<Item = char>) {
        let expected = chain.into_iter().collect::<Vec<_>>();
        let actual = graph
            .winning_chain()
            .into_iter()
            .map(|it| it.id)
            .collect::<Vec<_>>();
        assert_eq!(expected, actual)
    }

    #[test]
    fn single_block_is_winning_chain() {
        let graph = &mut TestBlockGraph::default();
        add_block(graph, None, 'a');
        assert_winning_chain(graph, ['a'])
    }

    #[test]
    fn smallest_block_is_winning_chain() {
        let graph = &mut TestBlockGraph::default();
        add_block(graph, None, 'a');
        add_block(graph, None, 'b');
        assert_winning_chain(graph, ['a']);

        let graph = &mut TestBlockGraph::default();
        add_block(graph, None, 'b');
        add_block(graph, None, 'a');
        assert_winning_chain(graph, ['a']);
    }

    #[test]
    fn simple_longest_chain_wins() {
        let graph = &mut TestBlockGraph::default();
        add_block(graph, None, 'a');
        add_block(graph, 'a', 'b');
        assert_winning_chain(graph, ['a', 'b']);

        add_block(graph, 'a', 'c');
        assert_winning_chain(graph, ['a', 'b']);

        add_block(graph, 'c', 'd');
        assert_winning_chain(graph, ['a', 'c', 'd']);
    }

    #[test]
    fn out_of_order_chain_overtakes() {
        let graph = &mut TestBlockGraph::default();
        add_block(graph, None, 'a');
        add_block(graph, 'a', 'b');
        assert_winning_chain(graph, ['a', 'b']);

        add_block(graph, 'c', 'd');
        assert_winning_chain(graph, ['a', 'b']);

        add_block(graph, 'a', 'c');
        assert_winning_chain(graph, ['a', 'c', 'd']);
    }
}
