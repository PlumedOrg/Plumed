// Plumed fork, written with Claude Code: glue between Grammalecte's JavaScript build (the
// files of grammalecte-files.txt, evaluated before this script) and the Rust grammar step.
// `__plumedFile(path)` is provided by Rust and returns a data file of resources/grammalecte/.
"use strict";

helpers.loadFile = function (sPath) {
    return __plumedFile(sPath);
};

conj.init(helpers.loadFile("grammalecte/fr/conj_data.json"));
phonet.init(helpers.loadFile("grammalecte/fr/phonet_data.json"));
mfsp.init(helpers.loadFile("grammalecte/fr/mfsp_data.json"));
gc_engine.load("JavaScript", "aHSL", "grammalecte/graphspell/_dictionaries");

// The default options of Grammalecte's Python package (gc_options.py), which the rule
// selection was measured with: the JavaScript defaults differ on esp, nbsp, unit, html and
// latex, and typography options can change which grammar error wins at a position.
gc_engine.setOptions(new Map(Object.entries({
    typo: true, apos: true, eepi: true, esp: true, tab: false, nbsp: true, tu: true, maj: true,
    minis: true, num: true, virg: true, poncfin: false, unit: true, nf: true, liga: false,
    mapos: false, chim: true, ocr: false, conf: true, loc: true, gn: true, infi: true,
    conj: true, ppas: true, imp: true, inte: true, vmode: true, bs: true, pleo: true,
    eleu: true, neg: false, redon1: false, redon2: false, date: true, mc: false,
    idrule: false, html: false, latex: true, md: true,
})));

// gc_engine.load logs its errors instead of throwing them.
const plumedSpellChecker = gc_engine.getSpellChecker();
if (!plumedSpellChecker) {
    throw new Error("Grammalecte did not load");
}

// Grammar errors of a paragraph, offsets in UTF-16 code units.
function plumedGrammarErrors(sText) {
    const lErrors = [];
    for (const oErr of gc_engine.parse(sText, "FR", false, null, false)) {
        lErrors.push({
            start: oErr.nStart,
            end: oErr.nEnd,
            rule_id: oErr.sRuleId,
            kind: oErr.sType,
            suggestions: oErr.aSuggestions,
        });
    }
    return JSON.stringify(lErrors);
}

// Spans [start, end] of the words the spell checker does not know, in UTF-16 code units.
function plumedUnknownWords(sText) {
    return JSON.stringify(plumedSpellChecker.parseParagraph(sText).map((d) => [d.nStart, d.nEnd]));
}
