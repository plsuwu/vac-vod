import html
import itertools
import re
import unicodedata

TEST_FILES = ["-3oTZS-brE8.en.vtt", "LWhmb1cbJ6Y.en.vtt", "xZCFvCmz7CA.en.vtt"]
TS = re.compile(r"<(\d{2}):(\d{2}):(\d{2})\.(\d{3})>")
CUE = re.compile(r"^(\d{2}):(\d{2}):(\d{2})\.(\d{3}) --> ")
TAGS = re.compile(r"</?c[^>]*>")
BRACKETED = re.compile(r"\[(music|laughter|__)\]", re.I)


def parse_cue_body(line: str, cue_start: float) -> list[tuple[float, str]]:
    line = TAGS.sub("", line)
    parts = [norm(p) for p in TS.split(line)]
    words = []

    if cue_pt := parts[0].strip():
        if len(cue_pt.split(" ")) == 1:
            words.append((cue_start, cue_pt))

    for i in range(1, len(parts), 5):
        h, m, s, ms = map(int, parts[i : i + 4])
        t = h * 3600 + m * 60 + s + ms / 1000
        text = parts[i + 4].strip()

        words.append((t, text))

        # if len(text) == 1:
        #     words.append((t, text))
        # else:
        #     for word in text:
        #         words.append((t, word))

    return force_monotonic(words)


def get_cue_start(line: str) -> float | None:
    if cue := CUE.findall(line):
        cue = cue[0]
        h, m, s, ms = map(int, cue[0:4])
        t = h * 3600 + m * 60 + s + ms / 1000

        return t
    return None


def clean_whitespace(lines: list[str]) -> list[str]:
    filtered = list(filter(lambda line: not line.isspace() and line != "", lines))

    return [line.strip() for line in filtered]


def norm(w: str) -> str:
    w = html.unescape(w)
    w = unicodedata.normalize("NFKC", w)

    return w.strip()


def collapse_runs(words: list[tuple[float, str]], max_reps=2):
    out, run = [], 0
    for t, w in words:
        key = w.lower().strip(".,!?")
        # print(w, "->", key)
        if out and key == out[-1][1].lower().strip(".,!?"):
            run += 1
            if run >= max_reps:
                continue
        else:
            run = 0
        out.append((t, w))

    return out


def force_monotonic(words: list[tuple[float, str]]):
    out, last = [], 0.0
    for t, w in words:
        if t < last:
            t = last

        out.append((t, w))
        last = t
    return out


def clean_subtitles(filepath: str) -> list[tuple[float, str]]:
    raw_content = open(filepath).readlines()[4:]
    content = clean_whitespace(raw_content)

    parsed_cues = []
    next_start = 0.0

    for line in content:
        if cue_start := get_cue_start(line):
            next_start = cue_start

        else:
            cue_body = parse_cue_body(line, next_start)
            if len(cue_body) > 1:
                cue_body = collapse_runs(cue_body)
                parsed_cues.append(cue_body)

    return list(itertools.chain.from_iterable(parsed_cues))
