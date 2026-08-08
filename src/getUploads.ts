import type {
	PlaylistIdPart,
	PlaylistItemListResponse,
} from "./types";
const { API_KEY } = Bun.env;

const API_URL = "https://www.googleapis.com/youtube/v3";
const DEFAULT_PARTS: Array<PlaylistIdPart> = [
	"contentDetails",
	"id",
	"snippet",
	"status",
];

function deriveChannelUploadsPlaylist(channelId: string) {
	if (channelId.startsWith("UC")) {
		return `UU${channelId.slice(2)}`;
	} else if (channelId.startsWith("UU")) {
		return channelId;
	}

	throw new Error("invalid channel id");
}

function buildPlaylistListQueryParams({
	playlistId,
	maxResults,
	part,
	nextPageToken,
}: {
	playlistId: string;
	maxResults?: number;
	part?: Array<PlaylistIdPart>;
	key?: string;
	nextPageToken?: string;
}) {
	const params = {
		playlistId,
		maxResults: String(maxResults ?? 50),
		part: part ? part.join(",") : DEFAULT_PARTS.join(","),
		key: API_KEY!,
	};

	if (nextPageToken != null) {
		(params as any).pageToken = nextPageToken;
	}

	return params;
}

async function handleFetch({
	channelId,
	part,
	maxResults,
	nextPageToken,
	apiUrl,
}: {
	channelId: string;
	part?: Array<PlaylistIdPart>;
	maxResults?: number;
	nextPageToken?: string;
	apiUrl?: string;
}) {
	const params = buildPlaylistListQueryParams({
		playlistId: deriveChannelUploadsPlaylist(channelId),
		part,
		maxResults,
		nextPageToken,
	});
	const endpoint = new URL(`${apiUrl ?? API_URL}/playlistItems`);
	for (const [k, v] of Object.entries(params)) {
		endpoint.searchParams.set(k, v);
	}

	try {
		const res = await fetch(endpoint);
		const body = await res.json();

		if (res.ok) {
			const parsed = parsePlaylistItemListResponse(
				body as PlaylistItemListResponse
			);

			return { success: true, data: parsed };
		} else {
			console.error("response non-200:", res.status, res.statusText);
			console.error(await res.json());

			return { success: false, data: null };
		}
	} catch (err) {
		console.error("failed:", err);
		return { success: false, data: null };
	}
}

function parsePlaylistItemListResponse(
	data: PlaylistItemListResponse
) {
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

export async function fetchPlaylists({
	channelId,
	apiUrl,
}: {
	channelId: string;
	apiUrl?: string;
}) {
	const playlistItems = new Array();

	let expectedLength = 0;
	let nextPageToken: string | undefined = undefined;

	do {
		const res = await handleFetch({
			channelId,
			apiUrl,
			nextPageToken,
		});
		if (!res.success || !res.data) {
			break;
		}

		expectedLength = res.data.pageInfo.totalResults;
		nextPageToken = res.data.nextPageToken;

		playlistItems.push(...res.data.videos);

		console.log(
			`${playlistItems.length} of ${expectedLength} total items...`
		);
	} while (playlistItems.length < expectedLength);

	return playlistItems;
}
