export interface Asset {
  name: string;
  browser_download_url: string;
  size: number;
}

export interface Release {
  tag_name: string;
  published_at: string;
  html_url: string;
  body: string;
  assets: Asset[];
}

export interface Contributor {
  total: number;
  weeks: Array<{ w: number; a: number; d: number; c: number }>;
  author: {
    login: string;
    id: number;
    avatar_url: string;
    html_url: string;
  };
}

export interface ChangelogPost {
  id: string;
  title: string;
  markdown: string;
  createdAt: string;
  updatedAt: string;
}
