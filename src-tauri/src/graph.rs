use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct CommitNode {
    pub id: String,
    pub parents: Vec<String>,
    pub summary: String,
    pub author: String,
    pub email: String,
    pub time: i64,
    pub refs: String,
    pub lane: usize,
    pub parent_lanes: Vec<usize>,
}

/// Carriles abiertos: el id que todavía tiene que aparecer en esa columna.
pub fn place(commits: &mut [CommitNode], open: &mut Vec<Option<String>>) {
    for commit in commits.iter_mut() {
        let lane = if let Some(i) = open.iter().position(|slot| slot.as_deref() == Some(commit.id.as_str()))
        {
            i
        } else if let Some(i) = open.iter().position(|slot| slot.is_none()) {
            i
        } else {
            open.push(None);
            open.len() - 1
        };
        commit.lane = lane;
        open[lane] = None;
        commit.parent_lanes.clear();
        for (index, parent) in commit.parents.iter().enumerate() {
            let preferred = if index == 0 { Some(lane) } else { None };
            let parent_lane = place_parent(open, parent, preferred);
            commit.parent_lanes.push(parent_lane);
        }
    }
}

fn place_parent(open: &mut Vec<Option<String>>, parent: &str, preferred: Option<usize>) -> usize {
    if let Some(i) = open.iter().position(|slot| slot.as_deref() == Some(parent)) {
        return i;
    }
    if let Some(pref) = preferred {
        if open.get(pref).is_some_and(|slot| slot.is_none()) {
            open[pref] = Some(parent.to_string());
            return pref;
        }
    }
    if let Some(i) = open.iter().position(|slot| slot.is_none()) {
        open[i] = Some(parent.to_string());
        return i;
    }
    open.push(Some(parent.to_string()));
    open.len() - 1
}
