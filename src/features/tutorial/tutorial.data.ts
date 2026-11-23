// Plumed fork (written with Claude Code): the spoken commands the tutorial
// lists. Every entry is a real default rule, checked against the example pairs
// of src-tauri/medical-data/interhop/rules.json (the InterHop rules seeded in
// the formatting rules). They are spoken French, so they are not translated.

export interface SpokenCommand {
    /** What to say. */
    say: string;
    /** What Plumed writes (`\n` is a line break). */
    writes: string;
}

export const LAYOUT_COMMANDS: SpokenCommand[] = [
    { say: 'à la ligne', writes: '\\n' },
    { say: 'saute une ligne', writes: '\\n' },
    { say: 'saut de ligne', writes: '\\n\\n (blank line)' },
    { say: 'nouveau tiret', writes: '- ' },
];

export const PUNCTUATION_COMMANDS: SpokenCommand[] = [
    { say: 'virgule', writes: ',' },
    { say: 'point', writes: '.' },
    { say: 'point final', writes: '.' },
    { say: 'deux points', writes: ':' },
    { say: 'point virgule', writes: ';' },
    { say: "point d'exclamation", writes: '!' },
    { say: 'trois points', writes: '...' },
    { say: 'ouvrez la parenthèse', writes: '(' },
    { say: 'fermez la parenthèse', writes: ')' },
    { say: 'ouvrez les guillemets', writes: '"' },
    { say: 'slash', writes: '/' },
    { say: "trait d'union", writes: '-' },
];

export const SHORTHAND_COMMANDS: SpokenCommand[] = [
    { say: '15 mg par kilo par jour', writes: '15 mg/kg/j' },
    { say: 'tension 120 80', writes: 'TA : 120/80' },
    { say: 'saturation 98', writes: 'SpO2 98%' },
    { say: 'pouls 75', writes: 'FC 75 bpm' },
    { say: '3 fois par jour', writes: '3/j' },
];

/** Whole-text templates: "Ajoute la commande <name>." writes a ready-made note. */
export const TEMPLATE_NAMES = [
    'examen clinique',
    'examen simplifié',
    'examen traumatisme cranien',
    'sortie contre avis',
];

/** A word alone is ambiguous for the model, a sentence gives it context. */
export const CONTEXT_EXAMPLE = {
    tooShort: 'venlafaxine',
    full: 'Le patient prend de la venlafaxine 75 mg le matin.',
};
