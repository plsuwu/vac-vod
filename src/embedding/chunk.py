from dataclasses import asdict, dataclass


@dataclass
class Chunked:
    channel_id: str
    video_id: str
    start: float
    end: float
    text: str


def windows(seg, size, stride):
    if len(seg) <= size:
        yield seg
        return

    for i in range(0, len(seg) - size, stride):
        yield seg[i : i + size]

    yield seg[len(seg) - size :]


def chunk_parts(
    channel_id, video_id, words, size=200, stride=100, max_gap=60.0
) -> list[Chunked]:
    segments, seg = [], [words[0]]
    for prev, cur in zip(words, words[1:]):
        if cur[0] - prev[0] > max_gap:
            segments.append(seg)
            seg = []
        seg.append(cur)
    segments.append(seg)

    chunks = []
    for seg in segments:
        for win in windows(seg, size, stride):
            chunk = Chunked(
                channel_id,
                video_id,
                start=win[0][0],
                end=win[-1][0],
                text=" ".join(w for _, w in win),
            )

            chunks.append(asdict(chunk))

    return chunks
