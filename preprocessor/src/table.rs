//! 出力対象テーブルのインメモリ表現。
//!
//! かつては PostgreSQL のテーブルへ取り込み、`information_schema` から列を引いて
//! すべて text へキャストして書き出していた。ここではその「列の並びと text 表現」を
//! そのまま持つ。値は原則として CSV のセルを素通しするため、取り込みと書き出しで
//! 表現がずれない。
//!
//! NULL と空文字は書き出し時にどちらも空文字になるので区別しない。

use std::collections::HashMap;

/// 1 セル。`None` は NULL を表す。
pub type Cell = Option<String>;

/// 列名の並びと行を持つだけの素朴なテーブル。
///
/// 主キーは呼び出し側が `pk` として列名で指定する。PostgreSQL の
/// `ON CONFLICT (pk) DO NOTHING` と同じく、既にある主キーの行は無視する。
pub struct Table {
    columns: Vec<String>,
    index_of: HashMap<String, usize>,
    rows: Vec<Vec<Cell>>,
    /// 主キー列の位置。`None` なら重複検査を行わない。
    pk: Option<usize>,
    /// 主キー値 -> 行番号。
    by_pk: HashMap<String, usize>,
}

impl Table {
    pub fn new(columns: &[&str], pk: Option<&str>) -> Self {
        let columns: Vec<String> = columns.iter().map(|c| (*c).to_string()).collect();
        let index_of = columns
            .iter()
            .enumerate()
            .map(|(i, c)| (c.clone(), i))
            .collect::<HashMap<_, _>>();
        let pk = pk.map(|name| index_of[name]);
        Table {
            columns,
            index_of,
            rows: Vec::new(),
            pk,
            by_pk: HashMap::new(),
        }
    }

    pub fn columns(&self) -> &[String] {
        &self.columns
    }

    pub fn rows(&self) -> &[Vec<Cell>] {
        &self.rows
    }

    /// 主キー索引を張り替えない書き換え用。主キー列は触らないこと。
    pub fn rows_mut(&mut self) -> &mut [Vec<Cell>] {
        &mut self.rows
    }

    pub fn len(&self) -> usize {
        self.rows.len()
    }

    /// 列名から位置を引く。存在しない列名はプログラムの誤りなので落とす。
    pub fn col(&self, name: &str) -> usize {
        *self
            .index_of
            .get(name)
            .unwrap_or_else(|| panic!("列 {name} が存在しない"))
    }

    /// すべて NULL の行を作る。呼び出し側が必要な列だけ埋める。
    pub fn blank_row(&self) -> Vec<Cell> {
        vec![None; self.columns.len()]
    }

    /// 行を追加する。主キーが既出なら追加せず `false` を返す
    /// (`ON CONFLICT DO NOTHING` 相当)。
    pub fn push(&mut self, row: Vec<Cell>) -> bool {
        debug_assert_eq!(row.len(), self.columns.len());
        if let Some(pk) = self.pk {
            let key = row[pk].clone().unwrap_or_default();
            if self.by_pk.contains_key(&key) {
                return false;
            }
            self.by_pk.insert(key, self.rows.len());
        }
        self.rows.push(row);
        true
    }

    /// 指定列で行を並べ替える。数値列として比較する。
    pub fn sort_by_int_col(&mut self, name: &str) {
        let idx = self.col(name);
        self.rows
            .sort_by_key(|row| cell_i64(row, idx).unwrap_or(i64::MIN));
        if let Some(pk) = self.pk {
            self.by_pk = self
                .rows
                .iter()
                .enumerate()
                .map(|(i, row)| (row[pk].clone().unwrap_or_default(), i))
                .collect();
        }
    }
}

pub fn cell_i32(row: &[Cell], idx: usize) -> Option<i32> {
    row[idx].as_deref().and_then(|v| v.trim().parse().ok())
}

pub fn cell_i64(row: &[Cell], idx: usize) -> Option<i64> {
    row[idx].as_deref().and_then(|v| v.trim().parse().ok())
}

/// 値を持つセルにする。空文字は NULL と区別しないのでそのまま入れる。
pub fn text(value: impl Into<String>) -> Cell {
    Some(value.into())
}

pub fn int(value: i32) -> Cell {
    Some(value.to_string())
}
