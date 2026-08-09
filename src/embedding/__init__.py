import glob
import json
import os
import pathlib

import jsonlines

from .chunk import Chunked, chunk_parts
from .clean import clean_subtitles


def ensure_output_dir(outdir: str, parent_path: str):

    pass


def get_subs_dir(
    channel_id: str, dir: str, raw_subdir: str = "raw"
) -> pathlib.Path | None:
    canonical_dir = (
        pathlib.Path(dir)
        .resolve()
        .joinpath(channel_id)
        .joinpath(raw_subdir)
    )
    if not os.path.exists(canonical_dir):
        print(
            f"cannot find raw subs for {channel_id} ({canonical_dir} not found)"
        )
        return None

    return canonical_dir


def get_sub_files(
    channel_id: str, dir="subs/", ext="vtt"
) -> list[str]:
    if canonical_dir := get_subs_dir(channel_id, dir):
        sub_files = glob.glob(f"{canonical_dir}/*.{ext}")
        video_ids = map(
            lambda n: n.split("/")[-1].split(".")[0], sub_files
        )

        return list(zip(video_ids, sub_files))
    return None


def write_jsonl(
    chunked_subs: list[Chunked],
    video_id: str,
    channel_id: str,
    dir="subs/",
):
    outdir = (
        pathlib.Path(dir)
        .resolve()
        .joinpath(channel_id)
        .joinpath("jsonl")
    )

    if not os.path.exists(outdir):
        os.mkdir(outdir)

    outfile = f"{outdir}/{video_id}.jsonl"
    with jsonlines.open(outfile, mode="w") as jf:
        jf.write_all(chunked_subs)

    print(f"ok: {video_id}")


def main() -> None:
    channel_ids = ["test"]
    for channel_id in channel_ids:
        files = get_sub_files(channel_id)
        for video_id, file in files:
            clean = clean_subtitles(file)
            chunks = chunk_parts(channel_id, video_id, clean)

            write_jsonl(chunks, video_id, channel_id)
