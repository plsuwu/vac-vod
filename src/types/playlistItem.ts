export const PLAYLIST_ID_PARTS = [
	"contentDetails",
	"id",
	"snippet",
	"status",
] as const;
export type PlaylistIdPart = (typeof PLAYLIST_ID_PARTS)[number];
export type Video = {
	title: string;
	videoId: string;
	publishedAt: Date;
	thumbnails:
		| {
				url: string;
				width: number;
				height: number;
		  }
		| undefined;
};

export type PlaylistItemListResponse = {
	kind: "youtube#playlistItemListResponse";
	etag: string;
	nextPageToken: string;
	prevPageToken: string;
	pageInfo: {
		totalResults: number;
		resultsPerPage: number;
	};
	items: Array<PlaylistItemResource>;
};

export type PlaylistItemSnippet =
	| {
			publishedAt: Date;
			channelId: string;
			title: string;
			description: string;
			thumbnails: {
				[key: string]: {
					url: string;
					width: number;
					height: number;
				};
			};
			channelTitle: string;
			videoOwnerChannelTitle: string;
			videoOwnerChannelId: string;
			playlistId: string;
			position: number;
			resourceId: {
				kind: string;
				videoId: string;
			};
	  }
	| undefined;

export type PlaylistQueryParams = {
	key: string;
	playlistId: string;
	maxResults: string;
	part: string;
	pageToken?: string;
};

export type PlaylistItemContentDetails =
	| {
			videoId: string;
			startAt: string;
			endAt: string;
			note: string;
			videoPublishedAt: Date;
	  }
	| undefined;

export type PlaylistItemStatus =
	| {
			privacyStatus: string;
	  }
	| undefined;

export type PlaylistItemResource = {
	kind: "youtube#playlistItem";
	etag: string;
	id?: string;
	snippet: PlaylistItemSnippet;
	contentDetails: PlaylistItemContentDetails;
	status: PlaylistItemStatus;
};
