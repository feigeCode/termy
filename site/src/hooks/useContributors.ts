import { useQuery, type UseQueryResult } from "@tanstack/react-query";

export interface ContributorWeek {
  w: number;
  a: number;
  d: number;
  c: number;
}

export interface ContributorAuthor {
  login: string;
  id: number;
  avatar_url: string;
  html_url: string;
}

export interface Contributor {
  total: number;
  weeks: ContributorWeek[];
  author: ContributorAuthor;
}

async function fetchContributors(): Promise<Contributor[]> {
  const res = await fetch("/api/github/contributors");

  if (!res.ok) {
    throw new Error(`Failed to load contributors (${res.status})`);
  }

  return res.json();
}

const CONTRIBUTORS_STALE_TIME = 5 * 60_000;
const CONTRIBUTORS_GC_TIME = 30 * 60_000;

export function useContributors(): UseQueryResult<Contributor[], Error> {
  return useQuery({
    queryKey: ["github-contributors"],
    queryFn: fetchContributors,
    staleTime: CONTRIBUTORS_STALE_TIME,
    gcTime: CONTRIBUTORS_GC_TIME,
  });
}
