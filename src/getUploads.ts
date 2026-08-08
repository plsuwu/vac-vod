import type {
	PlaylistItemListResponse,
	PlaylistQueryParams,
	Video,
	ChannelId,
	PlaylistId,
} from "./types";

export const BASE_API_URL = "https://www.googleapis.com/youtube/v3";

export class Playlist {
	private _apiKey: string;
	playlistItems: Array<Video> = new Array();
	fetchedItems = 0;
	totalItems = 0;
	apiUrl: URL;
	channelId: ChannelId;
	uploadsPlaylistId: string;
	params: PlaylistQueryParams;

	constructor({
		apiKey,
		channelId,
		apiUrl,
		playlistId,
	}: {
		apiKey: string;
		channelId: string;
		apiUrl?: string;
		playlistId?: string;
	}) {
		if (!Playlist.isChannelId(channelId)) {
			throw new Error(`invalid channelId: '${channelId}'`);
		}

		this.apiUrl = new URL(apiUrl ?? `${BASE_API_URL}/playlistItems`);
		this._apiKey = apiKey;
		this.channelId = channelId;
		this.uploadsPlaylistId =
			playlistId ?? Playlist.derivePlaylistId(this.channelId);

		this.params = {
			maxResults: "50",
			part: "contentDetails,snippet",
			playlistId: this.uploadsPlaylistId,
			key: this._apiKey,
		};
	}

	static isChannelId(channel: string): channel is ChannelId {
		return channel.startsWith("UC");
	}

	static derivePlaylistId(channel: ChannelId): PlaylistId {
		return `UU${channel.slice(2)}`;
	}

	setPage(pageToken: string | undefined = undefined) {
		this.params.pageToken = pageToken;
	}

	async getPlaylistItems() {
		do {
			const res = await this.getNextPage();
			if (!res.data) break;

			this.totalItems = res.data.pageInfo.totalResults;
			this.fetchedItems += res.data.videos.length;
			this.playlistItems.push(...res.data.videos);

			this.setPage(res.data.nextPageToken);
			console.log(
				`progress: ${this.playlistItems.length}/${this.totalItems} items`
			);
		} while (this.fetchedItems < this.totalItems);
	}

	async getNextPage() {
		const nextUrl = this.apiUrl;
		Object.entries(this.params).forEach(([k, v]) =>
			nextUrl.searchParams.set(k, v)
		);

		try {
			const res = await fetch(nextUrl);
			const body = await res.json();
			if (res.ok) {
				const parsed = this.parseResponse(body as PlaylistItemListResponse);

				return { data: parsed };
			} else {
				console.error("fetch failure:", res.status, res.statusText);
				console.error(await res.json());
				return { data: null };
			}
		} catch (e) {
			console.error("failed during page fetch:", e);
			return { data: null };
		}
	}

	parseResponse(data: PlaylistItemListResponse) {
		const { nextPageToken, items, pageInfo } = data;

		const videos = items.map((item) => {
			const { title, publishedAt, thumbnails } = item.snippet!;
			const { videoId } = item.snippet!.resourceId;

			return {
				title,
				videoId,
				publishedAt,
				thumbnails: thumbnails.standard ?? thumbnails.default,
			};
		});

		return { videos, nextPageToken, pageInfo };
	}
}
