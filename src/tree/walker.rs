//! Tree traversal utilities.

use super::node::{EmberTree, TreeNodeRef};

/// A walker for traversing the Ember+ tree.
pub struct TreeWalker<'a> {
    tree: &'a EmberTree,
    stack: Vec<(TreeNodeRef, Vec<i32>)>,
}

impl<'a> TreeWalker<'a> {
    /// Create a new walker starting from the root.
    pub fn new(tree: &'a EmberTree) -> Self {
        let stack: Vec<(TreeNodeRef, Vec<i32>)> = tree
            .children()
            .map(|node| {
                let number = node.read().number();
                (std::sync::Arc::clone(node), vec![number])
            })
            .collect();
        
        TreeWalker { tree, stack }
    }

    /// Get the next node and its path.
    pub fn next(&mut self) -> Option<(TreeNodeRef, Vec<i32>)> {
        let (node, path) = self.stack.pop()?;
        
        // Add children to stack
        let node_guard = node.read();
        for child in node_guard.children() {
            let child_number = child.read().number();
            let mut child_path = path.clone();
            child_path.push(child_number);
            self.stack.push((std::sync::Arc::clone(child), child_path));
        }
        drop(node_guard);
        
        Some((node, path))
    }
}

impl<'a> Iterator for TreeWalker<'a> {
    type Item = (TreeNodeRef, Vec<i32>);

    fn next(&mut self) -> Option<Self::Item> {
        TreeWalker::next(self)
    }
}

/// Find nodes by identifier.
pub fn find_by_identifier(tree: &EmberTree, identifier: &str) -> Vec<(TreeNodeRef, Vec<i32>)> {
    let mut results = Vec::new();
    
    for (node, path) in TreeWalker::new(tree) {
        let matches = {
            let guard = node.read();
            guard.identifier() == Some(identifier)
        };
        if matches {
            results.push((node, path));
        }
    }
    
    results
}

/// Find nodes by path pattern (supports wildcards).
pub fn find_by_pattern(tree: &EmberTree, pattern: &str) -> Vec<(TreeNodeRef, Vec<i32>)> {
    let parts: Vec<&str> = pattern.split('.').collect();
    let mut results = Vec::new();
    
    fn matches(path: &[i32], pattern: &[&str]) -> bool {
        if path.len() != pattern.len() {
            return false;
        }
        
        for (num, pat) in path.iter().zip(pattern.iter()) {
            if *pat != "*" {
                if let Ok(expected) = pat.parse::<i32>() {
                    if *num != expected {
                        return false;
                    }
                } else {
                    return false;
                }
            }
        }
        
        true
    }
    
    for (node, path) in TreeWalker::new(tree) {
        let path_strs: Vec<String> = path.iter().map(|n| n.to_string()).collect();
        let path_refs: Vec<&str> = path_strs.iter().map(|s| s.as_str()).collect();
        
        if matches(&path, &parts) {
            results.push((node, path));
        }
    }
    
    results
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tree::node::TreeNode;

    #[test]
    fn test_walker() {
        let mut tree = EmberTree::new();
        
        let mut root = TreeNode::new_node(0);
        let child1 = std::sync::Arc::new(parking_lot::RwLock::new(TreeNode::new_node(1)));
        let child2 = std::sync::Arc::new(parking_lot::RwLock::new(TreeNode::new_node(2)));
        root.add_child(child1);
        root.add_child(child2);
        
        tree.add_root_child(root);
        
        let mut count = 0;
        for _ in TreeWalker::new(&tree) {
            count += 1;
        }
        
        assert_eq!(count, 3); // root + 2 children
    }
}
