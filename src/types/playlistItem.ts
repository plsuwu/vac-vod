export const PLAYLIST_ID_PARTS = [
	"contentDetails",
	"id",
	"snippet",
	"status",
] as const;
export type PlaylistIdPart = (typeof PLAYLIST_ID_PARTS)[number];

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

export type PlaylistItemSnippet = {
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
} | undefined;

// export type PlaylistItemSnippet =
// 	| {
// 			channelTitle: string;
// 			videoOwnerChannelTitle: string;
// 			videoOwnerChannelId: string;
// 			playlistId: string;
// 			position: number; // uint
// 			resourceId: {
// 				kind: string;
// 				videoId: string;
// 			};
// 	  }
// 	| undefined;

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
