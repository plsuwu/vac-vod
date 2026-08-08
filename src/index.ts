import { $ } from "bun";
import { Playlist } from "./getUploads.ts";

const { API_KEY } = Bun.env;

for (const id of ["UCaZkRdEEpePJ4EEZznuqh8g", "UCBustguC_fsnqDQZxOBgGEg"]) {
	const playlistHandler = new Playlist({
		apiKey: API_KEY!,
		channelId: id,
	});

	const manifestFilepath = `subs/${playlistHandler.channelId}`;
	$`mkdir -p ${manifestFilepath}/.raw`;

	await playlistHandler.getPlaylistItems();
	const videos = playlistHandler.playlistItems;
	const manifestData = JSON.stringify(videos);

	await Bun.write(`${manifestFilepath}/manifest.json`, manifestData);
}
