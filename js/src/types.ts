// The document tree, the same shape docstream gives every client. See core/src/tree.rs.
export type Align = 'none' | 'left' | 'center' | 'right';
export type AlertKind = 'note' | 'tip' | 'important' | 'warning' | 'caution';
export type Inline =
    | { type: 'text'; text: string }
    | { type: 'strong' | 'emphasis' | 'strike' | 'highlight' | 'superscript' | 'subscript'; children: Inline[] }
    | { type: 'code'; text: string }
    | { type: 'math'; tex: string; display: boolean }
    | { type: 'link'; href: string; children: Inline[] }
    | { type: 'image'; src: string; alt: string }
    | { type: 'break' };
export interface Item { checked?: boolean; blocks: Block[] }
/** A table cell: the column it starts in; spans of one, `header: false` and `align: 'none'` are left out. */
export interface Cell { inlines: Inline[]; column: number; rowSpan?: number; colSpan?: number; header?: boolean; align?: Align }
export type Block =
    | { type: 'paragraph'; inlines: Inline[] }
    | { type: 'heading'; level: number; anchor: string; inlines: Inline[] }
    | { type: 'quote'; blocks: Block[] }
    | { type: 'alert'; kind: AlertKind; blocks: Block[] }
    | { type: 'list'; ordered: boolean; start: number; items: Item[] }
    | { type: 'code'; language: string; text: string }
    | { type: 'math'; tex: string }
    | { type: 'table'; columns: number; head: Cell[][]; body: Cell[][] }
    | { type: 'rule' }
    | { type: 'html'; text: string };
/** A top-level block: `key` changes only when the block does; `settled` once nothing more can join it. */
export interface Keyed { key: string; settled: boolean; block: Block }
/** The whole document as it now stands. */
export interface Update { blocks: Keyed[]; done: boolean }
