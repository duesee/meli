/*
 * meli
 *
 * Copyright 2017-2018 Manos Pitsidianakis
 *
 * This file is part of meli.
 *
 * meli is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * meli is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with meli. If not, see <http://www.gnu.org/licenses/>.
 */

use std::cmp;

use super::*;
use crate::components::PageMovement;

#[derive(Debug, Clone)]
struct ThreadEntry {
    index: (usize, ThreadNodeHash, usize),
    /// (indentation, thread_node index, line number in listing)
    indentation: usize,
    msg_hash: EnvelopeHash,
    seen: bool,
    dirty: bool,
    hidden: bool,
    heading: String,
    timestamp: UnixTimestamp,
}

#[derive(Debug, Default, Clone)]
pub struct ThreadView {
    new_cursor_pos: usize,
    cursor_pos: usize,
    expanded_pos: usize,
    new_expanded_pos: usize,
    reversed: bool,
    coordinates: (AccountHash, MailboxHash, usize),
    thread_group: ThreadHash,
    mailview: MailView,
    show_mailview: bool,
    show_thread: bool,
    entries: Vec<ThreadEntry>,
    visible_entries: Vec<Vec<usize>>,
    indentation_colors: [ThemeAttribute; 6],
    use_color: bool,

    movement: Option<PageMovement>,
    dirty: bool,
    content: CellBuffer,
    id: ComponentId,
}

impl ThreadView {
    /*
     * coordinates: (account index, mailbox_hash, root set thread_node index)
     * expanded_hash: optional position of expanded entry when we render the
     * threadview. Default  expanded message is the last one.
     * context: current context
     */
    pub fn new(
        coordinates: (AccountHash, MailboxHash, usize),
        thread_group: ThreadHash,
        expanded_hash: Option<ThreadNodeHash>,
        context: &Context,
    ) -> Self {
        let mut view = ThreadView {
            reversed: false,
            coordinates,
            thread_group,
            mailview: MailView::default(),
            show_mailview: true,
            show_thread: true,
            entries: Vec::new(),
            cursor_pos: 1,
            new_cursor_pos: 0,
            dirty: true,
            id: ComponentId::new_v4(),
            indentation_colors: [
                crate::conf::value(context, "mail.view.thread.indentation.a"),
                crate::conf::value(context, "mail.view.thread.indentation.b"),
                crate::conf::value(context, "mail.view.thread.indentation.c"),
                crate::conf::value(context, "mail.view.thread.indentation.d"),
                crate::conf::value(context, "mail.view.thread.indentation.e"),
                crate::conf::value(context, "mail.view.thread.indentation.f"),
            ],
            use_color: context.settings.terminal.use_color(),
            ..Default::default()
        };
        view.initiate(expanded_hash, context);
        view.new_cursor_pos = view.new_expanded_pos;
        view
    }

    pub fn update(&mut self, context: &Context) {
        if self.entries.is_empty() {
            return;
        }

        let old_entries = self.entries.clone();

        let old_focused_entry = if self.entries.len() > self.cursor_pos {
            Some(self.entries.remove(self.cursor_pos))
        } else {
            None
        };

        let old_expanded_entry = if self.entries.len() > self.expanded_pos {
            Some(self.entries.remove(self.expanded_pos))
        } else {
            None
        };

        let expanded_hash = old_expanded_entry.as_ref().map(|e| e.index.1);
        self.initiate(expanded_hash, context);

        let mut old_cursor = 0;
        let mut new_cursor = 0;
        loop {
            if old_cursor >= old_entries.len() || new_cursor >= self.entries.len() {
                break;
            }
            if old_entries[old_cursor].msg_hash == self.entries[new_cursor].msg_hash
                || old_entries[old_cursor].index == self.entries[new_cursor].index
                || old_entries[old_cursor].heading == self.entries[new_cursor].heading
            {
                self.entries[new_cursor].hidden = old_entries[old_cursor].hidden;
                old_cursor += 1;
                new_cursor += 1;
            } else {
                new_cursor += 1;
            }
            self.recalc_visible_entries();
        }

        if let Some(old_focused_entry) = old_focused_entry {
            if let Some(new_entry_idx) = self.entries.iter().position(|e| {
                e.msg_hash == old_focused_entry.msg_hash
                    || (e.index.1 == old_focused_entry.index.1
                        && e.index.2 == old_focused_entry.index.2)
            }) {
                self.cursor_pos = new_entry_idx;
            }
        }
        if let Some(old_expanded_entry) = old_expanded_entry {
            if let Some(new_entry_idx) = self.entries.iter().position(|e| {
                e.msg_hash == old_expanded_entry.msg_hash
                    || (e.index.1 == old_expanded_entry.index.1
                        && e.index.2 == old_expanded_entry.index.2)
            }) {
                self.expanded_pos = new_entry_idx;
            }
        }
        self.set_dirty(true);
    }

    fn initiate(&mut self, expanded_hash: Option<ThreadNodeHash>, context: &Context) {
        #[inline(always)]
        fn make_entry(
            i: (usize, ThreadNodeHash, usize),
            msg_hash: EnvelopeHash,
            seen: bool,
            timestamp: UnixTimestamp,
        ) -> ThreadEntry {
            let (ind, _, _) = i;
            ThreadEntry {
                index: i,
                indentation: ind,
                msg_hash,
                seen,
                dirty: true,
                hidden: false,
                heading: String::new(),
                timestamp,
            }
        }

        let account = &context.accounts[&self.coordinates.0];
        let threads = account.collection.get_threads(self.coordinates.1);

        if !threads.groups.contains_key(&self.thread_group) {
            return;
        }

        let thread_iter = threads.thread_group_iter(self.thread_group);
        self.entries.clear();
        for (line, (ind, thread_node_hash)) in thread_iter.enumerate() {
            let entry = if let Some(msg_hash) = threads.thread_nodes()[&thread_node_hash].message()
            {
                let env_ref = account.collection.get_env(msg_hash);
                make_entry(
                    (ind, thread_node_hash, line),
                    msg_hash,
                    env_ref.is_seen(),
                    env_ref.timestamp,
                )
            } else {
                continue;
            };
            self.entries.push(entry);
            match expanded_hash {
                Some(expanded_hash) if expanded_hash == thread_node_hash => {
                    self.new_expanded_pos = self.entries.len().saturating_sub(1);
                    self.expanded_pos = self.new_expanded_pos + 1;
                }
                _ => {}
            }
        }
        if expanded_hash.is_none() {
            self.new_expanded_pos = self
                .entries
                .iter()
                .enumerate()
                .reduce(|a, b| if a.1.timestamp > b.1.timestamp { a } else { b })
                .map(|el| el.0)
                .unwrap_or(0);
            self.expanded_pos = self.new_expanded_pos + 1;
        }

        let height = 2 * self.entries.len() + 1;
        let mut width = 0;

        let mut highlight_reply_subjects: Vec<Option<usize>> =
            Vec::with_capacity(self.entries.len());
        for e in &mut self.entries {
            let envelope: EnvelopeRef = context.accounts[&self.coordinates.0]
                .collection
                .get_env(e.msg_hash);
            let thread_node = &threads.thread_nodes()[&e.index.1];
            let string = if thread_node.show_subject() {
                let subject = envelope.subject();
                highlight_reply_subjects.push(Some(subject.grapheme_width()));
                format!(
                    "  {} - {} {}{}",
                    envelope.date_as_str(),
                    envelope.field_from_to_string(),
                    envelope.subject(),
                    if envelope.has_attachments() {
                        " 📎"
                    } else {
                        ""
                    },
                )
            } else {
                highlight_reply_subjects.push(None);
                format!(
                    "  {} - {}{}",
                    envelope.date_as_str(),
                    envelope.field_from_to_string(),
                    if envelope.has_attachments() {
                        " 📎"
                    } else {
                        ""
                    },
                )
            };
            e.heading = string;
            width = cmp::max(width, e.index.0 * 4 + e.heading.grapheme_width() + 2);
        }
        let theme_default = crate::conf::value(context, "theme_default");
        let highlight_theme = crate::conf::value(context, "highlight");
        let mut content = CellBuffer::new_with_context(width, height, None, context);
        if self.reversed {
            for (y, e) in self.entries.iter().rev().enumerate() {
                /* Box character drawing stuff */
                if y > 0 && content.get_mut(e.index.0 * 4, 2 * y - 1).is_some() {
                    let index = (e.index.0 * 4, 2 * y - 1);
                    if content[index].ch() == ' ' {
                        let mut ctr = 1;
                        while content.get(e.index.0 * 4 + ctr, 2 * y - 1).is_some() {
                            if content[(e.index.0 * 4 + ctr, 2 * y - 1)].ch() != ' ' {
                                break;
                            }
                            set_and_join_box(
                                &mut content,
                                (e.index.0 * 4 + ctr, 2 * y - 1),
                                BoxBoundary::Horizontal,
                            );
                            ctr += 1;
                        }
                        set_and_join_box(&mut content, index, BoxBoundary::Horizontal);
                    }
                }
                write_string_to_grid(
                    &e.heading,
                    &mut content,
                    if e.seen {
                        theme_default.fg
                    } else {
                        highlight_theme.fg
                    },
                    if e.seen {
                        theme_default.bg
                    } else {
                        highlight_theme.bg
                    },
                    theme_default.attrs,
                    (
                        (e.index.0 * 4 + 1, 2 * y),
                        (e.index.0 * 4 + e.heading.grapheme_width() + 1, height - 1),
                    ),
                    None,
                );
                if let Some(len) = highlight_reply_subjects[y] {
                    let index = e.index.0 * 4 + 1 + e.heading.grapheme_width() - len;
                    let area = ((index, 2 * y), (width - 2, 2 * y));
                    change_colors(&mut content, area, highlight_theme.fg, theme_default.bg);
                }
                set_and_join_box(&mut content, (e.index.0 * 4, 2 * y), BoxBoundary::Vertical);
                set_and_join_box(
                    &mut content,
                    (e.index.0 * 4, 2 * y + 1),
                    BoxBoundary::Vertical,
                );
                for i in ((e.index.0 * 4) + 1)..width - 1 {
                    set_and_join_box(&mut content, (i, 2 * y + 1), BoxBoundary::Horizontal);
                }
                set_and_join_box(&mut content, (width - 1, 2 * y), BoxBoundary::Vertical);
                set_and_join_box(&mut content, (width - 1, 2 * y + 1), BoxBoundary::Vertical);
            }
        } else {
            for (y, e) in self.entries.iter().enumerate() {
                /* Box character drawing stuff */
                let mut x = 0;
                for i in 0..e.index.0 {
                    let att =
                        self.indentation_colors[(i).wrapping_rem(self.indentation_colors.len())];
                    change_colors(
                        &mut content,
                        ((x, 2 * y), (x + 3, 2 * y + 1)),
                        att.fg,
                        att.bg,
                    );
                    x += 4;
                }
                if y > 0 && content.get_mut(e.index.0 * 4, 2 * y - 1).is_some() {
                    let index = (e.index.0 * 4, 2 * y - 1);
                    if content[index].ch() == ' ' {
                        let mut ctr = 1;
                        content[(e.index.0 * 4, 2 * y - 1)].set_bg(theme_default.bg);
                        while content.get(e.index.0 * 4 + ctr, 2 * y - 1).is_some() {
                            content[(e.index.0 * 4 + ctr, 2 * y - 1)].set_bg(theme_default.bg);
                            if content[(e.index.0 * 4 + ctr, 2 * y - 1)].ch() != ' ' {
                                break;
                            }
                            set_and_join_box(
                                &mut content,
                                (e.index.0 * 4 + ctr, 2 * y - 1),
                                BoxBoundary::Horizontal,
                            );
                            ctr += 1;
                        }
                        set_and_join_box(&mut content, index, BoxBoundary::Horizontal);
                    }
                }
                write_string_to_grid(
                    &e.heading,
                    &mut content,
                    if e.seen {
                        theme_default.fg
                    } else {
                        highlight_theme.fg
                    },
                    if e.seen {
                        theme_default.bg
                    } else {
                        highlight_theme.bg
                    },
                    theme_default.attrs,
                    (
                        (e.index.0 * 4 + 1, 2 * y),
                        (e.index.0 * 4 + e.heading.grapheme_width() + 1, height - 1),
                    ),
                    None,
                );
                if let Some(_len) = highlight_reply_subjects[y] {
                    let index = e.index.0 * 4 + 1;
                    let area = ((index, 2 * y), (width - 2, 2 * y));
                    change_colors(&mut content, area, highlight_theme.fg, theme_default.bg);
                }
                set_and_join_box(&mut content, (e.index.0 * 4, 2 * y), BoxBoundary::Vertical);
                set_and_join_box(
                    &mut content,
                    (e.index.0 * 4, 2 * y + 1),
                    BoxBoundary::Vertical,
                );
                for i in ((e.index.0 * 4) + 1)..width - 1 {
                    set_and_join_box(&mut content, (i, 2 * y + 1), BoxBoundary::Horizontal);
                }
                set_and_join_box(&mut content, (width - 1, 2 * y), BoxBoundary::Vertical);
                set_and_join_box(&mut content, (width - 1, 2 * y + 1), BoxBoundary::Vertical);
            }

            for y in 0..height - 1 {
                set_and_join_box(&mut content, (width - 1, y), BoxBoundary::Vertical);
            }
        }
        self.content = content;
        self.visible_entries = vec![(0..self.entries.len()).collect()];
    }

    fn highlight_line(
        &self,
        grid: &mut CellBuffer,
        dest_area: Area,
        src_area: Area,
        idx: usize,
        context: &Context,
    ) {
        let visibles: Vec<&usize> = self.visible_entries.iter().flat_map(|v| v.iter()).collect();
        if idx == *visibles[self.cursor_pos] {
            let theme_default = crate::conf::value(context, "theme_default");
            let bg_color = crate::conf::value(context, "highlight").bg;
            let attrs = if self.use_color {
                theme_default.attrs
            } else {
                Attr::REVERSE
            };
            for row in grid.bounds_iter(dest_area) {
                for c in row {
                    grid[c]
                        .set_fg(theme_default.fg)
                        .set_bg(bg_color)
                        .set_attrs(attrs);
                }
            }
            change_colors(grid, dest_area, theme_default.fg, bg_color);
            return;
        }

        copy_area(grid, &self.content, dest_area, src_area);
    }

    fn draw_list(&mut self, grid: &mut CellBuffer, area: Area, context: &mut Context) {
        let (upper_left, bottom_right) = area;
        let (width, height) = self.content.size();
        if height == 0 {
            context.dirty_areas.push_back(area);
            return;
        }
        let rows = (get_y(bottom_right) - get_y(upper_left)).wrapping_div(2);
        if rows == 0 {
            context.dirty_areas.push_back(area);
            return;
        }
        if let Some(mvm) = self.movement.take() {
            match mvm {
                PageMovement::Up(amount) => {
                    self.new_cursor_pos = self.new_cursor_pos.saturating_sub(amount);
                }
                PageMovement::PageUp(multiplier) => {
                    self.new_cursor_pos = self.new_cursor_pos.saturating_sub(rows * multiplier);
                }
                PageMovement::Down(amount) => {
                    if self.new_cursor_pos + amount + 1 < height {
                        self.new_cursor_pos += amount;
                    } else if self.new_cursor_pos + amount > height {
                        self.new_cursor_pos = height - 1;
                    } else {
                        self.new_cursor_pos = (height / rows) * rows;
                    }
                }
                PageMovement::PageDown(multiplier) => {
                    if self.new_cursor_pos + rows * multiplier + 1 < height {
                        self.new_cursor_pos += rows * multiplier;
                    } else {
                        self.new_cursor_pos = (height / rows) * rows;
                    }
                }
                PageMovement::Right(_) | PageMovement::Left(_) => {}
                PageMovement::Home => {
                    self.new_cursor_pos = 0;
                }
                PageMovement::End => {
                    self.new_cursor_pos = (height / rows) * rows;
                }
            }
        }
        if self.new_cursor_pos >= self.entries.len() {
            self.new_cursor_pos = self.entries.len().saturating_sub(1);
        }
        let prev_page_no = (self.cursor_pos).wrapping_div(rows);
        let page_no = (self.new_cursor_pos).wrapping_div(rows);

        let top_idx = page_no * rows;
        /* returns the **line** of an entry in the ThreadView grid. */
        let get_entry_area = |idx: usize, entries: &[ThreadEntry]| {
            let entries = &entries;
            let visual_indentation = entries[idx].index.0 * 4;
            (
                (visual_indentation, 2 * idx),
                (
                    visual_indentation + entries[idx].heading.grapheme_width() + 1,
                    2 * idx,
                ),
            )
        };

        if self.dirty || (page_no != prev_page_no) {
            if page_no != prev_page_no {
                clear_area(grid, area, crate::conf::value(context, "theme_default"));
            }
            let visibles: Vec<&usize> =
                self.visible_entries.iter().flat_map(|v| v.iter()).collect();

            for (visible_entry_counter, v) in visibles.iter().skip(top_idx).take(rows).enumerate() {
                if visible_entry_counter >= rows {
                    break;
                }
                let idx = *v;
                copy_area(
                    grid,
                    &self.content,
                    (
                        pos_inc(upper_left, (0, 2 * visible_entry_counter)), // dest_area
                        bottom_right,
                    ),
                    (
                        (0, 2 * idx), //src_area
                        (width - 1, 2 * idx + 1),
                    ),
                );
            }
            /* If cursor position has changed, remove the highlight from the previous
             * position and apply it in the new one. */
            self.cursor_pos = self.new_cursor_pos;
            if self.cursor_pos + 1 > visibles.len() {
                self.cursor_pos = visibles.len().saturating_sub(1);
            }
            let idx = *visibles[self.cursor_pos];
            let src_area = { get_entry_area(idx, &self.entries) };
            let visual_indentation = self.entries[idx].indentation * 4;
            let dest_area = (
                pos_inc(
                    upper_left,
                    (visual_indentation, 2 * (self.cursor_pos - top_idx)),
                ),
                (
                    cmp::min(
                        get_x(bottom_right),
                        get_x(upper_left)
                            + visual_indentation
                            + self.entries[idx].heading.grapheme_width()
                            + 1,
                    ),
                    cmp::min(
                        get_y(bottom_right),
                        get_y(upper_left) + 2 * (self.cursor_pos - top_idx),
                    ),
                ),
            );

            self.highlight_line(grid, dest_area, src_area, idx, context);
            if rows < visibles.len() {
                ScrollBar::default().set_show_arrows(true).draw(
                    grid,
                    (
                        pos_inc(upper_left!(area), (width!(area).saturating_sub(1), 0)),
                        bottom_right,
                    ),
                    context,
                    2 * self.cursor_pos,
                    rows,
                    2 * visibles.len() + 1,
                );
            }
            if 2 * top_idx + rows > 2 * visibles.len() + 1 {
                clear_area(
                    grid,
                    (
                        pos_inc(upper_left, (0, 2 * (visibles.len() - top_idx) + 1)),
                        bottom_right,
                    ),
                    crate::conf::value(context, "theme_default"),
                );
            }
            context.dirty_areas.push_back(area);
        } else {
            let old_cursor_pos = self.cursor_pos;
            self.cursor_pos = self.new_cursor_pos;
            /* If cursor position has changed, remove the highlight from the previous
             * position and apply it in the new one. */
            let visibles: Vec<&usize> =
                self.visible_entries.iter().flat_map(|v| v.iter()).collect();
            for &idx in &[old_cursor_pos, self.cursor_pos] {
                let entry_idx = *visibles[idx];
                let src_area = { get_entry_area(entry_idx, &self.entries) };
                let visual_indentation = self.entries[entry_idx].indentation * 4;
                let dest_area = (
                    pos_inc(
                        upper_left,
                        (visual_indentation, 2 * (visibles[..idx].len() - top_idx)),
                    ),
                    (
                        cmp::min(
                            get_x(bottom_right),
                            get_x(upper_left)
                                + visual_indentation
                                + self.entries[entry_idx].heading.grapheme_width()
                                + 1,
                        ),
                        cmp::min(
                            get_y(bottom_right),
                            get_y(upper_left) + 2 * (visibles[..idx].len() - top_idx),
                        ),
                    ),
                );

                self.highlight_line(grid, dest_area, src_area, entry_idx, context);
                if rows < visibles.len() {
                    ScrollBar::default().set_show_arrows(true).draw(
                        grid,
                        (
                            pos_inc(upper_left!(area), (width!(area).saturating_sub(1), 0)),
                            bottom_right,
                        ),
                        context,
                        2 * self.cursor_pos,
                        rows,
                        2 * visibles.len() + 1,
                    );
                    context.dirty_areas.push_back((
                        upper_left!(area),
                        set_x(bottom_right, get_x(upper_left!(area)) + 1),
                    ));
                }

                let (upper_left, bottom_right) = dest_area;
                context
                    .dirty_areas
                    .push_back((upper_left, (get_x(bottom_right), get_y(upper_left) + 1)));
            }
        }
    }

    fn draw_vert(&mut self, grid: &mut CellBuffer, area: Area, context: &mut Context) {
        let upper_left = upper_left!(area);

        let bottom_right = bottom_right!(area);
        let mid = get_x(upper_left) + self.content.size().0;

        let theme_default = crate::conf::value(context, "theme_default");
        /* First draw the thread subject on the first row */
        let y = if self.dirty {
            clear_area(grid, area, theme_default);
            let account = &context.accounts[&self.coordinates.0];
            let threads = account.collection.get_threads(self.coordinates.1);
            let thread_root = threads
                .thread_group_iter(self.thread_group)
                .next()
                .unwrap()
                .1;
            let thread_node = &threads.thread_nodes()[&thread_root];
            let i = thread_node.message().unwrap_or_else(|| {
                let mut iter_ptr = thread_node.children()[0];
                while threads.thread_nodes()[&iter_ptr].message().is_none() {
                    iter_ptr = threads.thread_nodes()[&iter_ptr].children()[0];
                }
                threads.thread_nodes()[&iter_ptr].message().unwrap()
            });
            let envelope: EnvelopeRef = account.collection.get_env(i);

            let (x, y) = write_string_to_grid(
                &envelope.subject(),
                grid,
                crate::conf::value(context, "highlight").fg,
                theme_default.bg,
                theme_default.attrs,
                area,
                Some(get_x(upper_left)),
            );
            for x in x..=get_x(bottom_right) {
                grid[(x, y)]
                    .set_ch(' ')
                    .set_fg(theme_default.fg)
                    .set_bg(theme_default.bg);
            }
            context
                .dirty_areas
                .push_back((upper_left, set_y(bottom_right, y + 1)));
            context
                .dirty_areas
                .push_back(((mid, y + 1), set_x(bottom_right, mid)));
            clear_area(
                grid,
                ((mid, y + 1), set_x(bottom_right, mid)),
                theme_default,
            );
            y + 2
        } else {
            get_y(upper_left) + 2
        };
        let (width, height) = self.content.size();
        if height == 0 || width == 0 {
            return;
        }
        for x in get_x(upper_left)..=get_x(bottom_right) {
            set_and_join_box(grid, (x, y - 1), BoxBoundary::Horizontal);
            grid[(x, y - 1)]
                .set_fg(theme_default.fg)
                .set_bg(theme_default.bg);
        }

        match (self.show_mailview, self.show_thread) {
            (true, true) => {
                self.draw_list(
                    grid,
                    (set_y(upper_left, y), set_x(bottom_right, mid - 1)),
                    context,
                );
                let upper_left = (mid + 1, get_y(upper_left) + y - 1);
                self.mailview
                    .draw(grid, (upper_left, bottom_right), context);
            }
            (false, true) => {
                clear_area(
                    grid,
                    ((mid + 1, get_y(upper_left) + y - 1), bottom_right),
                    theme_default,
                );
                self.draw_list(grid, (set_y(upper_left, y), bottom_right), context);
            }
            (_, false) => {
                self.mailview.draw(grid, area, context);
            }
        }
    }

    fn draw_horz(&mut self, grid: &mut CellBuffer, area: Area, context: &mut Context) {
        let upper_left = upper_left!(area);
        let bottom_right = bottom_right!(area);
        let total_rows = height!(area);

        let pager_ratio = *mailbox_settings!(
            context[self.coordinates.0][&self.coordinates.1]
                .pager
                .pager_ratio
        );
        let mut bottom_entity_rows = (pager_ratio * total_rows) / 100;

        if bottom_entity_rows > total_rows {
            bottom_entity_rows = total_rows.saturating_sub(1);
        }

        let mut mid = get_y(upper_left) + total_rows - bottom_entity_rows;
        if mid >= get_y(bottom_right) {
            mid = get_y(bottom_right) / 2;
        }
        let mid = mid;

        let theme_default = crate::conf::value(context, "theme_default");
        /* First draw the thread subject on the first row */
        let y = {
            clear_area(grid, area, theme_default);
            let account = &context.accounts[&self.coordinates.0];
            let threads = account.collection.get_threads(self.coordinates.1);
            let thread_root = threads
                .thread_group_iter(self.thread_group)
                .next()
                .unwrap()
                .1;
            let thread_node = &threads.thread_nodes()[&thread_root];
            let i = thread_node.message().unwrap_or_else(|| {
                let mut iter_ptr = thread_node.children()[0];
                while threads.thread_nodes()[&iter_ptr].message().is_none() {
                    iter_ptr = threads.thread_nodes()[&iter_ptr].children()[0];
                }
                threads.thread_nodes()[&iter_ptr].message().unwrap()
            });
            let envelope: EnvelopeRef = account.collection.get_env(i);

            let (x, y) = write_string_to_grid(
                &envelope.subject(),
                grid,
                theme_default.fg,
                theme_default.bg,
                theme_default.attrs,
                area,
                Some(get_x(upper_left)),
            );
            for x in x..=get_x(bottom_right) {
                grid[(x, y)]
                    .set_ch(' ')
                    .set_fg(theme_default.fg)
                    .set_bg(theme_default.bg);
            }
            context
                .dirty_areas
                .push_back((upper_left, set_y(bottom_right, y + 2)));
            y + 2
        };

        for x in get_x(upper_left)..=get_x(bottom_right) {
            set_and_join_box(grid, (x, y - 1), BoxBoundary::Horizontal);
            grid[(x, y - 1)]
                .set_fg(theme_default.fg)
                .set_bg(theme_default.bg);
        }

        let (width, height) = self.content.size();
        if height == 0 || height == self.cursor_pos || width == 0 {
            return;
        }

        clear_area(
            grid,
            (set_y(upper_left, y), set_y(bottom_right, mid + 1)),
            theme_default,
        );
        let (width, height) = self.content.size();

        match (self.show_mailview, self.show_thread) {
            (true, true) => {
                let area = (set_y(upper_left, y), set_y(bottom_right, mid));
                let upper_left = upper_left!(area);
                let bottom_right = bottom_right!(area);

                let rows = (get_y(bottom_right).saturating_sub(get_y(upper_left) + 1)) / 2;
                if rows == 0 {
                    return;
                }
                let page_no = (self.new_cursor_pos).wrapping_div(rows);
                let top_idx = page_no * rows;

                copy_area(
                    grid,
                    &self.content,
                    area,
                    ((0, 2 * top_idx), (width - 1, height - 1)),
                );
                context.dirty_areas.push_back(area);
            }
            (false, true) => {
                let area = (set_y(upper_left, y), bottom_right);
                let upper_left = upper_left!(area);

                let rows = (get_y(bottom_right).saturating_sub(get_y(upper_left) + 1)) / 2;
                if rows == 0 {
                    return;
                }
                let page_no = (self.new_cursor_pos).wrapping_div(rows);
                let top_idx = page_no * rows;
                copy_area(
                    grid,
                    &self.content,
                    area,
                    ((0, 2 * top_idx), (width - 1, height - 1)),
                );
                context.dirty_areas.push_back(area);
            }
            (_, false) => { /* show only envelope */ }
        }

        match (self.show_mailview, self.show_thread) {
            (true, true) => {
                let area = (set_y(upper_left, mid), set_y(bottom_right, mid));
                context.dirty_areas.push_back(area);
                for x in get_x(upper_left)..=get_x(bottom_right) {
                    set_and_join_box(grid, (x, mid), BoxBoundary::Horizontal);
                    grid[(x, mid)]
                        .set_fg(theme_default.fg)
                        .set_bg(theme_default.bg);
                }
                let area = (set_y(upper_left, y), set_y(bottom_right, mid - 1));
                self.draw_list(grid, area, context);
                self.mailview
                    .draw(grid, (set_y(upper_left, mid + 1), bottom_right), context);
            }
            (false, true) => {
                self.dirty = true;
                self.draw_list(grid, (set_y(upper_left, y), bottom_right), context);
            }
            (_, false) => {
                self.mailview.draw(grid, area, context);
            }
        }
    }

    fn recalc_visible_entries(&mut self) {
        if self
            .entries
            .iter_mut()
            .fold(false, |flag, e| e.dirty || flag)
        {
            self.visible_entries = self
                .entries
                .iter()
                .enumerate()
                .fold(
                    (vec![Vec::new()], SmallVec::<[_; 8]>::new(), false),
                    |(mut visies, mut stack, is_prev_hidden), (idx, e)| {
                        match (e.hidden, is_prev_hidden) {
                            (true, false) => {
                                visies.last_mut().unwrap().push(idx);
                                stack.push(e.indentation);
                                (visies, stack, e.hidden)
                            }
                            (true, true)
                                if !stack.is_empty() && stack[stack.len() - 1] == e.indentation =>
                            {
                                visies.push(vec![idx]);
                                (visies, stack, e.hidden)
                            }
                            (true, true) => (visies, stack, e.hidden),
                            (false, true)
                                if stack[stack.len() - 1] >= e.indentation
                                    && stack.len() > 1
                                    && stack[stack.len() - 2] >= e.indentation =>
                            {
                                //FIXME pop all until e.indentation
                                visies.push(vec![idx]);
                                stack.pop();
                                (visies, stack, e.hidden)
                            }
                            (false, true) if stack[stack.len() - 1] >= e.indentation => {
                                visies.push(vec![idx]);
                                stack.pop();
                                (visies, stack, e.hidden)
                            }
                            (false, true) => (visies, stack, is_prev_hidden),
                            (false, false) => {
                                visies.last_mut().unwrap().push(idx);
                                (visies, stack, e.hidden)
                            }
                        }
                    },
                )
                .0;
        }
        if self.reversed {
            self.visible_entries.reverse()
        }
    }

    /// Current position in self.entries (not in drawn entries which might
    /// exclude nonvisible ones)
    fn current_pos(&self) -> usize {
        let visibles: Vec<&usize> = self.visible_entries.iter().flat_map(|v| v.iter()).collect();
        *visibles[self.new_cursor_pos]
    }
}

impl fmt::Display for ThreadView {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "view thread")
    }
}

impl Component for ThreadView {
    fn draw(&mut self, grid: &mut CellBuffer, area: Area, context: &mut Context) {
        let total_cols = width!(area);
        if self.entries.is_empty() {
            self.dirty = false;
            return;
        }
        if !self.is_dirty() {
            return;
        }

        /* If user has selected another mail to view, change to it */
        if self.new_expanded_pos != self.expanded_pos {
            self.expanded_pos = self.new_expanded_pos;
            let coordinates = (
                self.coordinates.0,
                self.coordinates.1,
                self.entries[self.current_pos()].msg_hash,
            );
            self.mailview.update(coordinates, context);
        }

        if self.entries.len() == 1 {
            self.mailview.draw(grid, area, context);
        } else if total_cols >= self.content.size().0 + 74 {
            self.draw_vert(grid, area, context);
        } else {
            self.draw_horz(grid, area, context);
        }
        self.dirty = false;
    }

    fn process_event(&mut self, event: &mut UIEvent, context: &mut Context) -> bool {
        if let UIEvent::Action(Listing(OpenInNewTab)) = event {
            /* Handle this before self.mailview does */
            context
                .replies
                .push_back(UIEvent::Action(Tab(New(Some(Box::new(self.clone()))))));
            return true;
        }

        if self.show_mailview && self.mailview.process_event(event, context) {
            return true;
        }

        let shortcuts = self.get_shortcuts(context);
        match *event {
            UIEvent::Input(ref key)
                if shortcut!(key == shortcuts[Shortcuts::THREAD_VIEW]["scroll_up"]) =>
            {
                if self.cursor_pos > 0 {
                    self.new_cursor_pos = self.new_cursor_pos.saturating_sub(1);
                    self.dirty = true;
                }
                return true;
            }
            UIEvent::Input(ref key)
                if shortcut!(key == shortcuts[Shortcuts::THREAD_VIEW]["scroll_down"]) =>
            {
                let height = self.visible_entries.iter().flat_map(|v| v.iter()).count();
                if height > 0 && self.new_cursor_pos + 1 < height {
                    self.new_cursor_pos += 1;
                    self.dirty = true;
                }
                return true;
            }
            UIEvent::Input(ref key)
                if shortcut!(key == shortcuts[Shortcuts::THREAD_VIEW]["prev_page"]) =>
            {
                self.movement = Some(PageMovement::PageUp(1));
                self.dirty = true;
            }
            UIEvent::Input(ref key)
                if shortcut!(key == shortcuts[Shortcuts::THREAD_VIEW]["next_page"]) =>
            {
                self.movement = Some(PageMovement::PageDown(1));
                self.dirty = true;
            }
            UIEvent::Input(ref key) if *key == Key::Home => {
                self.movement = Some(PageMovement::Home);
                self.dirty = true;
            }
            UIEvent::Input(ref key) if *key == Key::End => {
                self.movement = Some(PageMovement::End);
                self.dirty = true;
            }
            UIEvent::Input(Key::Char('\n')) => {
                if self.entries.len() < 2 {
                    return true;
                }
                self.new_expanded_pos = self.current_pos();
                self.show_mailview = true;
                self.set_dirty(true);
                return true;
            }
            UIEvent::Input(ref key)
                if shortcut!(key == shortcuts[Shortcuts::THREAD_VIEW]["toggle_mailview"]) =>
            {
                self.show_mailview = !self.show_mailview;
                self.set_dirty(true);
                return true;
            }
            UIEvent::Input(ref key)
                if shortcut!(key == shortcuts[Shortcuts::THREAD_VIEW]["toggle_threadview"]) =>
            {
                self.show_thread = !self.show_thread;
                self.set_dirty(true);
                return true;
            }
            UIEvent::Input(ref key)
                if shortcut!(key == shortcuts[Shortcuts::THREAD_VIEW]["reverse_thread_order"]) =>
            {
                self.reversed = !self.reversed;
                let expanded_hash = self.entries[self.expanded_pos].index.1;
                self.initiate(Some(expanded_hash), context);
                self.dirty = true;
                return true;
            }
            UIEvent::Input(ref key)
                if shortcut!(key == shortcuts[Shortcuts::THREAD_VIEW]["collapse_subtree"]) =>
            {
                let current_pos = self.current_pos();
                self.entries[current_pos].hidden = !self.entries[current_pos].hidden;
                self.entries[current_pos].dirty = true;
                {
                    let visible_entries: Vec<&usize> =
                        self.visible_entries.iter().flat_map(|v| v.iter()).collect();
                    /* search_old_cursor_pos */
                    self.new_cursor_pos = (|entries: Vec<&usize>, x: usize| {
                        let mut low = 0;
                        let mut high = entries.len() - 1;
                        while low <= high {
                            let mid = low + (high - low) / 2;
                            if *entries[mid] == x {
                                return mid;
                            }
                            if x > *entries[mid] {
                                low = mid + 1;
                            } else {
                                high = mid - 1;
                            }
                        }
                        high + 1 //mid
                    })(visible_entries, self.cursor_pos);
                }
                self.cursor_pos = self.new_cursor_pos;
                self.recalc_visible_entries();
                self.dirty = true;
                return true;
            }
            UIEvent::Resize => {
                self.set_dirty(true);
            }
            UIEvent::EnvelopeRename(ref old_hash, ref new_hash) => {
                let account = &context.accounts[&self.coordinates.0];
                for e in self.entries.iter_mut() {
                    if e.msg_hash == *old_hash {
                        e.msg_hash = *new_hash;
                        let seen: bool = account.collection.get_env(*new_hash).is_seen();
                        if seen != e.seen {
                            self.dirty = true;
                        }
                        e.seen = seen;
                    }
                }
                self.mailview
                    .process_event(&mut UIEvent::EnvelopeRename(*old_hash, *new_hash), context);
            }
            UIEvent::EnvelopeUpdate(ref env_hash) => {
                let account = &context.accounts[&self.coordinates.0];
                for e in self.entries.iter_mut() {
                    if e.msg_hash == *env_hash {
                        let seen: bool = account.collection.get_env(*env_hash).is_seen();
                        if seen != e.seen {
                            self.dirty = true;
                        }
                        e.seen = seen;
                    }
                }
                self.mailview
                    .process_event(&mut UIEvent::EnvelopeUpdate(*env_hash), context);
            }
            _ => {
                if self.mailview.process_event(event, context) {
                    return true;
                }
            }
        }
        false
    }

    fn is_dirty(&self) -> bool {
        self.dirty || (self.show_mailview && self.mailview.is_dirty())
    }

    fn set_dirty(&mut self, value: bool) {
        self.dirty = value;
        self.mailview.set_dirty(value);
    }

    fn get_shortcuts(&self, context: &Context) -> ShortcutMaps {
        let mut map = self.mailview.get_shortcuts(context);

        map.insert(
            Shortcuts::THREAD_VIEW,
            context.settings.shortcuts.thread_view.key_values(),
        );

        map
    }

    fn id(&self) -> ComponentId {
        self.id
    }

    fn set_id(&mut self, id: ComponentId) {
        self.id = id;
    }

    fn kill(&mut self, id: ComponentId, context: &mut Context) {
        debug_assert!(self.id == id);
        context
            .replies
            .push_back(UIEvent::Action(Tab(Kill(self.id))));
    }
}
