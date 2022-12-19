import { useCallback, useEffect, useRef, useState } from "react";

export const BASE = "http://localhost:3005";

export type Trigger = () => void;
export function useTriggeredFetch<T>(path: string): [null | T, Trigger] {
  const [state, setState] = useState(null);
  const version = useRef(0);

  const doFetch = useCallback(async () => {
    const curVersion = ++version.current;
    setState(null);
    const fetched = await request(path);

    if(curVersion === version.current) setState(fetched);
  }, [path]);

  useEffect(() => {
    doFetch();
  }, [doFetch]);
  return [state, doFetch];
} 

export async function request(path: string, method: 'GET' | 'POST' | 'DELETE' = 'GET', body?: any): Promise<any> {
  const headers = new Headers();
  if(body !== undefined)
    headers.append('Content-Type', 'application/json');

  const resp = await fetch(`${BASE}${path}`, {
    headers,
    body: body ? JSON.stringify(body) : undefined,
    method,
  });

  if(resp.headers.get('Content-Type')?.startsWith('application/json')) return await resp.json();
  else return await resp.text();
}