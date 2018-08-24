/*
 * meli - ui crate.
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

/*!
  Components are ways to handle application data. They can draw on the terminal and receive events, but also do other stuff as well. (For example, see the `notifications` module.)

  See the `Component` Trait for more details.
  */

use super::*;

pub mod mail;
pub use mail::*;

pub mod notifications;

pub mod utilities;
pub use self::utilities::*;

use std::fmt;
use std::fmt::{Debug, Display};
use std::ops::Deref;

use super::{Key, UIEvent, UIEventType};
/// The upper and lower boundary char.
const HORZ_BOUNDARY: char = '─';
/// The left and right boundary char.
const VERT_BOUNDARY: char = '│';

/// The top-left corner
const _TOP_LEFT_CORNER: char = '┌';
/// The top-right corner
const _TOP_RIGHT_CORNER: char = '┐';
/// The bottom-left corner
const _BOTTOM_LEFT_CORNER: char = '└';
/// The bottom-right corner
const _BOTTOM_RIGHT_CORNER: char = '┘';

const LIGHT_VERTICAL_AND_RIGHT: char = '├';

const _LIGHT_VERTICAL_AND_LEFT: char = '┤';

const LIGHT_DOWN_AND_HORIZONTAL: char = '┬';

const LIGHT_UP_AND_HORIZONTAL: char = '┴';

/// `Entity` is a container for Components. Totally useless now so if it is not useful in the
/// future (ie hold some information, id or state) it should be removed.
#[derive(Debug)]
pub struct Entity {
    //context: VecDeque,
    pub component: Box<Component>, // more than one?
}

impl Display for Entity {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Display::fmt(&self.component, f)
    }
}

impl Deref for Entity {
    type Target = Box<Component>;

    fn deref(&self) -> &Box<Component> {
        &self.component
    }
}

impl Entity {
    /// Pass events to child component.
    pub fn rcv_event(&mut self, event: &UIEvent, context: &mut Context) -> bool {
        self.component.process_event(&event, context)
    }
}

/// Types implementing this Trait can draw on the terminal and receive events.
/// If a type wants to skip drawing if it has not changed anything, it can hold some flag in its
/// fields (eg self.dirty = false) and act upon that in their `draw` implementation.
pub trait Component: Display + Debug {
    fn draw(&mut self, grid: &mut CellBuffer, area: Area, context: &mut Context);
    fn process_event(&mut self, event: &UIEvent, context: &mut Context) -> bool;
    fn is_dirty(&self) -> bool {
        true
    }
    fn set_dirty(&mut self);
}

fn new_draft(_context: &mut Context) -> Vec<u8> {
    // TODO: Generate proper message-id https://www.jwz.org/doc/mid.html
    let mut v = String::with_capacity(500);
    v.push_str("From: \n");
    v.push_str("To: \n");
    v.push_str("Subject: \n");
    v.push_str("Message-Id: \n\n");
    v.into_bytes()
}

fn bin_to_ch(b: u32) -> char {
    match b {
        0b0001 => '╶',
        0b0010 => '╵',
        0b0011 => '└',
        0b0100 => '╴',
        0b0101 => '─',
        0b0110 => '┘',
        0b0111 => '┴',
        0b1000 => '╷',
        0b1001 => '┌',
        0b1010 => '│',
        0b1011 => '├',
        0b1100 => '┐',
        0b1101 => '┬',
        0b1110 => '┤',
        0b1111 => '┼',
        x => unreachable!(format!("unreachable bin_to_ch(x), x = {:b}", x)),
    }
}

fn ch_to_bin(ch: char) -> Option<u32> {
    match ch {
        '└' => Some(0b0011),
        '─' => Some(0b0101),
        '┘' => Some(0b0110),
        '┴' => Some(0b0111),
        '┌' => Some(0b1001),

        '│' => Some(0b1010),

        '├' => Some(0b1011),
        '┐' => Some(0b1100),
        '┬' => Some(0b1101),

        '┤' => Some(0b1110),

        '┼' => Some(0b1111),
        '╷' => Some(0b1000),

        '╵' => Some(0b0010),
        '╴' => Some(0b0100),
        '╶' => Some(0b0001),
        _ => None,
    }
}

#[allow(never_loop)]
fn set_and_join_vert(grid: &mut CellBuffer, idx: Pos) -> u32 {
    let (x, y) = idx;
    let mut bin_set = 0b1010;
    /* Check left side
     *
     *        1
     *   -> 2 │ 0
     *        3
     */
    loop {
        if x > 0 {
            if let Some(cell) = grid.get_mut(x - 1, y) {
                if let Some(adj) = ch_to_bin(cell.ch()) {
                    if (adj & 0b0001) > 0 {
                        bin_set |= 0b0100;
                        break;
                    } else if adj == 0b0100 {
                        cell.set_ch(bin_to_ch(0b0101));
                        bin_set |= 0b0100;
                        break;
                    }
                }
            }
        }
        bin_set &= 0b1011;
        break;
    }

    /* Check right side
     *
     *        1
     *      2 │ 0 <-
     *        3
     */
    loop {
        if let Some(cell) = grid.get_mut(x + 1, y) {
            if let Some(adj) = ch_to_bin(cell.ch()) {
                if (adj & 0b0100) > 0 {
                    bin_set |= 0b0001;
                    break;
                }
            }
        }
        bin_set &= 0b1110;
        break;
    }

    /* Set upper side
     *
     *        1 <-
     *      2 │ 0
     *        3
     */
    loop {
        if y > 0 {
            if let Some(cell) = grid.get_mut(x, y - 1) {
                if let Some(adj) = ch_to_bin(cell.ch()) {
                    cell.set_ch(bin_to_ch(adj | 0b1000));
                    break;
                }
            }
        }
        bin_set &= 0b1101;
        break;
    }

    /* Set bottom side
     *
     *        1
     *      2 │ 0
     *        3 <-
     */
    loop {
        if let Some(cell) = grid.get_mut(x, y + 1) {
            if let Some(adj) = ch_to_bin(cell.ch()) {
                cell.set_ch(bin_to_ch(adj | 0b0010));
                break;
            }
        }
        bin_set &= 0b0111;
        break;
    }

    if bin_set == 0 {
        bin_set = 0b1010;
    }

    bin_set
}

#[allow(never_loop)]
fn set_and_join_horz(grid: &mut CellBuffer, idx: Pos) -> u32 {
    let (x, y) = idx;
    let mut bin_set = 0b0101;
    /* Check upper side
     *
     *        1 <-
     *      2 ─ 0
     *        3
     */
    loop {
        if y > 0 {
            if let Some(cell) = grid.get_mut(x, y - 1) {
                if let Some(adj) = ch_to_bin(cell.ch()) {
                    if (adj & 0b1000) > 0 {
                        bin_set |= 0b0010;
                        break;
                    } else if adj == 0b0010 {
                        bin_set |= 0b0010;
                        cell.set_ch(bin_to_ch(0b1010));
                        break;
                    }
                }
            }
        }
        bin_set &= 0b1101;
        break;
    }

    /* Check bottom side
     *
     *        1
     *      2 ─ 0
     *        3 <-
     */
    loop {
        if let Some(cell) = grid.get_mut(x, y + 1) {
            if let Some(adj) = ch_to_bin(cell.ch()) {
                if (adj & 0b0010) > 0 {
                    bin_set |= 0b1000;
                    break;
                } else if adj == 0b1000 {
                    cell.set_ch(bin_to_ch(0b1010));
                    break;
                }
            }
        }
        bin_set &= 0b0111;
        break;
    }

    /* Set left side
     *
     *        1
     *   -> 2 ─ 0
     *        3
     */
    loop {
        if x > 0 {
            if let Some(cell) = grid.get_mut(x - 1, y) {
                if let Some(adj) = ch_to_bin(cell.ch()) {
                    cell.set_ch(bin_to_ch(adj | 0b0001));
                    break;
                }
            }
        }
        bin_set &= 0b1011;
        break;
    }

    /* Set right side
     *
     *        1
     *      2 ─ 0 <-
     *        3
     */
    loop {
        if let Some(cell) = grid.get_mut(x + 1, y) {
            if let Some(adj) = ch_to_bin(cell.ch()) {
                cell.set_ch(bin_to_ch(adj | 0b0100));
                break;
            }
        }
        bin_set &= 0b1110;
        break;
    }

    if bin_set == 0 {
        bin_set = 0b0101;
    }

    bin_set
}

fn set_and_join_box(grid: &mut CellBuffer, idx: Pos, ch: char) {
    /* Connected sides:
     *
     *        1
     *      2 c 0
     *        3
     *
     *     #3210
     *    0b____
     */

    let bin_set = match ch {
        '│' => set_and_join_vert(grid, idx),
        '─' => set_and_join_horz(grid, idx),
        _ => unreachable!(),
    };

    grid[idx].set_ch(bin_to_ch(bin_set));
}
