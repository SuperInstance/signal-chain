"""Tests for signal_chain_dial.py — the model-vs-code dial (the thesis's
Python reference implementation)."""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))

from signal_chain_dial import ModelVsCodeDial


class _Msg:
    def __init__(self, author, text):
        self.author = author
        self.text = text


class _Room:
    def __init__(self, messages):
        self.messages = messages


def _code_room():
    return _Room([
        _Msg("bot", "fix: handle null pointer in parser"),
        _Msg("bot", "def process(x): return x * 2"),
        _Msg("bot", "Traceback (most recent call last): KeyError: 'x'"),
        _Msg("dev", "impl SignalNode for Gain { fn process(&mut self, input: f64) -> f64 { input * self.amount } }"),
    ])


def _prose_room():
    return _Room([
        _Msg("writer", "I think the room holds something warm — we built it together and it remembers us."),
        _Msg("writer", "Perhaps the elephant is not something you see, but something you feel when you walk in."),
        _Msg("critic", "However, I wonder whether any of us noticed it before tonight. It seems, in a sense, alive."),
    ])


def test_code_heavy_room_reads_negative():
    r = ModelVsCodeDial().read(_code_room())
    assert isinstance(r, float)
    assert r < 0.0, r


def test_prose_heavy_room_reads_positive():
    r = ModelVsCodeDial().read(_prose_room())
    assert isinstance(r, float)
    assert r > 0.0, r


def test_code_room_is_more_negative_than_prose_room():
    code = ModelVsCodeDial().read(_code_room())
    prose = ModelVsCodeDial().read(_prose_room())
    assert code < prose, (code, prose)


def test_empty_room_is_neutral():
    assert ModelVsCodeDial().read(_Room([])) == 0.0


def test_reading_is_bounded():
    for room in (_code_room(), _prose_room(), _Room([])):
        r = ModelVsCodeDial().read(room)
        assert -1.0 <= r <= 1.0, r


def test_duck_typed_list_of_messages():
    # A bare iterable of (author, text) tuples works too.
    r = ModelVsCodeDial().read([("bot", "fix: null pointer"), ("bot", "def f(): return 1")])
    assert r < 0.0, r


def test_satisfies_elephant_dial_abc_if_available():
    try:
        from elephant.dial import Dial
    except ImportError:
        Dial = None

    d = ModelVsCodeDial()
    assert d.name == "model_vs_code"
    assert d.description
    r = d.read(_prose_room())
    assert isinstance(r, float)
    assert -1.0 <= r <= 1.0

    if Dial is not None:
        # The elephant's Dial ABC: read(room) -> float, with name/description.
        assert isinstance(d, Dial), type(d)
        assert callable(getattr(d, "read", None))


if __name__ == "__main__":
    fns = [test_code_heavy_room_reads_negative, test_prose_heavy_room_reads_positive,
           test_code_room_is_more_negative_than_prose_room, test_empty_room_is_neutral,
           test_reading_is_bounded, test_duck_typed_list_of_messages,
           test_satisfies_elephant_dial_abc_if_available]
    for fn in fns:
        fn()
        print(f"PASS {fn.__name__}")
    print("\nAll signal_chain_dial tests passed.")
