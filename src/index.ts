import { $ } from "bun";
import { Playlist } from "./getUploads.ts";

const { API_KEY } = Bun.env;

const playlistHandler = new Playlist({
	apiKey: API_KEY!,
	channelId: "UCaZkRdEEpePJ4EEZznuqh8g",
});

const manifestFilepath = `subs/${playlistHandler.channelId}`;
$`mkdir -p ${manifestFilepath}`;

await playlistHandler.getPlaylistItems();
const videos = playlistHandler.playlistItems;
const manifestData = JSON.stringify(videos);

await Bun.write(`${manifestFilepath}/manifest.json`, manifestData);
