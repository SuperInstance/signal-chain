"""signal_chain_dial — the model-vs-code dial, in one file.

The signal-chain thesis made executable: a room's signal is not only *what*
is said, but *who or what* generates it — a model thinking in the open, or
code executing deterministically. This file is a self-contained, dependency-
free reference implementation of the elephant's `ModelVsCodeDial`
(`elephant/dials/model_vs_code.py`). It can be dropped straight into an
elephant `DialBank` (it subclasses `elephant.dial.Dial` when that is
importable), or used standalone against any duck-typed room.

Reading: `[-1 code .. +1 model]` — `-1` = a pure code room (commits, diffs,
error logs), `+1` = a pure model room (prose, hedges, reflection), `0.0` =
empty or neutral. Lexicon + symbol scoring, pure stdlib.
"""
from __future__ import annotations

import re

WORD_RE = re.compile(r"\w+")

# --------------------------------------------------------------------------- #
# Lexicons — the words that smell like model vs code.                         #
# --------------------------------------------------------------------------- #

# Model: hedges, reflection, first-person, prose, creativity, warmth.
MODEL_WORDS = {
    "i", "we", "my", "our", "me", "us", "you", "your",
    "maybe", "perhaps", "probably", "likely", "arguably", "possibly",
    "feel", "felt", "feels", "feeling", "think", "thinks", "believe",
    "wonder", "wondered", "imagine", "remember", "remembers", "sense",
    "seemed", "seems", "seem", "however", "moreover", "therefore", "thus",
    "indeed", "ultimately", "meanwhile", "furthermore", "nevertheless",
    "story", "voice", "warm", "warmth", "light", "gentle", "soft", "alive",
    "holds", "held", "together", "kind", "wonderful", "beautiful",
    "something", "someone", "everything", "nothing", "ourselves", "myself",
}
MODEL_PHRASES = {
    "i think", "i believe", "i wonder", "i feel", "it seems", "in a sense",
    "sort of", "kind of", "as if", "what if", "to me", "for me",
    "maybe we", "perhaps the", "we are", "we were",
}

# Code: keywords, determinism, diffs, errors, commit discipline.
CODE_WORDS = {
    "def", "fn", "function", "return", "import", "class", "struct",
    "impl", "let", "const", "var", "pub", "match", "enum", "trait",
    "elif", "else", "loop", "while", "typeof", "interface", "namespace",
    "static", "void", "mut", "traceback", "error", "exception", "assert",
    "undefined", "nan", "null", "none", "todo", "fixme", "hack",
    "deprecated", "refactor", "merge", "commit", "push", "rebase", "pull",
    "diff", "patch", "lint", "typecheck", "coverage", "dockerfile",
    "pipeline", "syntaxerror", "keyerror", "typeerror",
}
CODE_PHRASES = {
    "feat:", "fix:", "chore:", "docs:", "refactor:", "test:", "perf:",
    "build:", "ci:", "revert:", "style:", "release:", "merge ", "commit ",
    "push ", "pull request", "diff --git", "+++ b/", "--- a/", "@@ -",
    "at line", "syntax error", "merge conflict", "type error",
    "null pointer", "undefined behavior", "running tests",
}
# Symbols that read as code: braces, brackets, parens, semicolons, operators.
CODE_SYMBOLS = re.compile(
    r"[{}()\[\];]|->|=>|::|==|!=|<=|>=|\+=|-=|\*=|/=|&&|\|\|"
)


def _score(text: str, words) -> float:
    """Score one message on the model/code spectrum, in [-1, +1].

    `text` is the lowercased raw message; `words` is an iterable of its
    lowercase `\\w+` tokens. Counts model vs code lexicon hits (words +
    phrases) plus code symbol density, then maps the balance onto [-1, +1]:
    -1 = pure code, +1 = pure model, 0 = no signal (or perfectly balanced).
    """
    wset = set(words)
    model = sum(1 for w in wset if w in MODEL_WORDS)
    model += sum(1 for p in MODEL_PHRASES if p in text)
    code = sum(1 for w in wset if w in CODE_WORDS)
    code += sum(1 for p in CODE_PHRASES if p in text)
    code += len(CODE_SYMBOLS.findall(text))
    total = model + code
    if total == 0:
        return 0.0
    return (model - code) / total


def _message_text_words(m) -> tuple:
    """Extract (lowercased_text, lowercase_word_tokens) from a message.

    Accepts an object with `.text` (and optionally `.words`), or a bare
    `(author, text)` tuple/list.
    """
    if isinstance(m, (tuple, list)):
        text = m[1] if len(m) > 1 else (m[0] if m else "")
    else:
        text = getattr(m, "text", "") or ""
    text = str(text)
    words = getattr(m, "words", None)
    if words is None:
        words = WORD_RE.findall(text.lower())
    return text.lower(), words


# Subclass the elephant's Dial ABC when it's importable; otherwise fall back
# to a minimal stand-in so this file works with zero dependencies.
try:
    from elephant.dial import Dial
except ImportError:  # pragma: no cover - standalone mode
    class Dial:
        name = "dial"
        description = ""

        def read(self, room):
            raise NotImplementedError


class ModelVsCodeDial(Dial):
    name = "model_vs_code"
    description = "who is generating the room's signal, [-1 code .. +1 model]"

    def read(self, room) -> float:
        """Read the model/code ratio of a room, in [-1, +1].

        `room` is an elephant `Room` (with `.messages`), any object exposing
        `.messages`, or a bare iterable of duck-typed messages (objects with
        `.author` + `.text`, or `(author, text)` tuples).
        """
        messages = getattr(room, "messages", None)
        if messages is None:
            messages = list(room) if room is not None else []
        if not messages:
            return 0.0
        scores = []
        for m in messages:
            text, words = _message_text_words(m)
            scores.append(_score(text, words))
        return max(-1.0, min(1.0, sum(scores) / len(scores)))
