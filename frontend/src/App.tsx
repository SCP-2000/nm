import React, { MouseEventHandler, useCallback, useEffect, useState } from 'react';
import logo from './logo.svg';
import './App.css';
import { request, useTriggeredFetch } from './utils';
import clsx from 'clsx';

import './style.scss';
import { link } from 'fs';

type Tab = 'Address' | 'Link' | 'Route';

function TabHandle({ self, cur, setTab }: { self: Tab, cur: Tab, setTab: (t: Tab) => void }): JSX.Element {
  return <div
    className={clsx('tab', { active: cur === self })}
    onClick={() => setTab(self)}
  >{ self }</div>
}

type Address = {
  family: number,
  plen: number,
  flags: number,
  scope: number,
  index: number,

  address: null | string,
  local: null | string,
  label: null | string,
  broadcast: null | string,
}

type Kind = 
  'Dummy' |
  'Ifb' |
  'Bridge' |
  'Tun' |
  'Vrf' |
  'Wireguard' |
  'Other';

type Link = {
  family: number,
  index: number,
  linklayer: number,
  flags: number,

  ifname: string | null,
  mtu: number | null,
  kind: Kind | null,
}

type Route = {
  family: number,
  table: number,
  scope: number,
  proto: number,

  dst: [string, number] | null,
  src: [string, number] | null,
  gateway: string | null,
  dev: number | null,
  prefsrc: string | null;
  metric: number | null;
}

function Attr({ name, value }: { name: string, value?: null | string | number }): JSX.Element | null {
  if(value === null || value === undefined || value === '') return null;

  return <div className='group'>
    <label>{ name }</label>
    <span>{ value }</span>
  </div>
}

function formatFamily(family: number): string {
  if(family === 2) return `AF_INET`;
  else if(family === 10) return `AF_INET6`;
  return `Other(${family})`;
}

function formatScope(scope: number): string {
  if(scope === 0) return 'Global';
  if(scope === 254) return 'Host';
  if(scope === 253) return 'Link';
  return `Other(${scope})`;
}

function fuckItGetInput(name: string): string {
  return document.querySelector<HTMLInputElement | HTMLSelectElement>(`input[name=${name}], select[name=${name}]`)!.value;
}

function fuckItGetInputOptional(name: string): string | null {
  const input = fuckItGetInput(name);
  if(input === '') return null;
  return input;
}

function exitDialog(setter: (shown: boolean) => void): (e: React.MouseEvent) => void {
  return (e) => {
    const target = e.target as HTMLElement;
    if(target.classList.contains('backdrop')) setter(false);
  }
}

function App() {
  const [addresses, updateAddresses] = useTriggeredFetch<Address[]>("/address");
  const [links, updateLinks] = useTriggeredFetch<Link[]>("/link");
  const [routes, updateRoutes] = useTriggeredFetch<Route[]>("/route");

  const [showAddAddress, setShowAddAddress] = useState(false);
  const [showAddLink, setShowAddLink] = useState(false);
  const [showAddRoute, setShowAddRoute] = useState(false);

  const [tab, setTab] = useState<Tab>("Address");

  const updateAll = useCallback(() => {
    updateAddresses();
    updateLinks();
    updateRoutes();
  }, [updateAddresses, updateLinks, updateRoutes]);

  if(links === null || addresses === null || routes === null) return null;

  function findLinkName(index: number): string | null {
    return links?.find(e => e.index === index)?.ifname ?? null;
  }

  const addressMain = <main>
    <button onClick={() => setShowAddAddress(true)}>Add</button>
    {(
      addresses.map(
        (addr, idx) =>
        <div className='box' key={idx}>
          <div className="title">
            { addr.address ?? '?' }
            <span className="sep">/</span>
            <span className="plen">{ addr.plen }</span>
            <span className="sep">@</span>
            <span className="eth">{ findLinkName(addr.index) ?? '?' }({addr.index})</span>
          </div>
          <Attr name="Label" value={addr.label} />
          <Attr name="Local" value={addr.local} />
          <Attr name="Broadcast" value={addr.broadcast} />

          <div className="small-attrs">
            <Attr name="family" value={formatFamily(addr.family)}></Attr>
            <Attr name="flags" value={addr.flags}></Attr>
            <Attr name="scope" value={formatScope(addr.scope)}></Attr>
          </div>

          <div className="actions">
            <button onClick={() => {
              request('/address', 'DELETE', addr).then(() => updateAll());
            }}>Delete</button>
          </div>
        </div>
      )
    )}
  </main>;

  const linkMain = <main>
    <button onClick={() => setShowAddLink(true)}>Add</button>
    {(
      links.map(
        (link, idx) =>
        <div className='box' key={idx}>
          <div className="title">
            { link.ifname ?? '?' }
            <span className="index">({ link.index })</span>
            { link.kind && <span className="kind">:&nbsp;{link.kind}</span> }
          </div>
          <Attr name="MTU" value={link.mtu} />

          <div className="small-attrs">
            <Attr name="flags" value={link.flags}></Attr>
            <Attr name="linklayer" value={link.linklayer}></Attr>
          </div>

          <div className="actions">
            <button onClick={() => {
              request('/link', 'DELETE', link).then(() => updateAll());
            }}>Delete</button>
          </div>
        </div>
      )
    )}
  </main>;

  const routeMain = <main>
    <button onClick={() => setShowAddRoute(true)}>Add</button>
    {(
      routes.map(
        (route, idx) =>
        <div className='box' key={idx}>
          <div className="title route">
            <span className="block">
              { route.src ? `${route.src[0]}/${route.src[1]}` : '*' }
              <span className='sep'>-&gt;</span>
              { route.dst ?  `${route.dst[0]}/${route.dst[1]}` : '*' }
            </span>

            <span className='sep wide'>=&gt;</span>

            <span className="block">
              {route.dev !== null ? findLinkName(route.dev) : null}({route.dev})
              {
                route.gateway !== null && <>
                  <span className="sep">via</span>
                  {route.gateway}
                </>
              }
            </span>
          </div>

          <Attr name="Metric" value={route.metric} />
          <Attr name="Prefsrc" value={route.prefsrc} />

          <div className="small-attrs">
            <Attr name="family" value={formatFamily(route.family)}></Attr>
            <Attr name="table" value={route.table}></Attr>
            <Attr name="scope" value={formatScope(route.scope)}></Attr>
            <Attr name="proto" value={route.proto}></Attr>
          </div>

          <div className="actions">
            <button onClick={() => {
              request('/route', 'DELETE', route).then(() => updateAll());
            }}>Delete</button>
          </div>
        </div>
      )
    )}
  </main>;

  return <>
    <header>
      <nav>
        <TabHandle self="Address" cur={tab} setTab={setTab} />
        <TabHandle self="Link" cur={tab} setTab={setTab} />
        <TabHandle self="Route" cur={tab} setTab={setTab} />
      </nav>
      {
        tab === 'Address' ? addressMain
        : tab === 'Link' ? linkMain
        : routeMain
      }
    </header>

    <div className={clsx("backdrop", { shown: showAddRoute })} onClick={exitDialog(setShowAddRoute)}>
      <div className="dialog">
        <div className="dialog-title">Add route</div>
        <div className="input-hint">From</div>
        <input placeholder='*' name="route-src" />

        <div className="input-hint">To</div>
        <input placeholder='*' name="route-dst" />

        <div className="input-hint">Gateway</div>
        <input placeholder='None' name="route-gateway" />

        <div className="input-hint">Device</div>
        <select name="route-dev">
          {links.map((l, idx) => <option key={idx} value={l.index}>{l.ifname}({l.index})</option>)}
        </select>

        <div className="input-hint">Table</div>
        <input placeholder='254' name="route-table" />

        <div className="input-hint">Metric</div>
        <input placeholder='None' name="route-metric" />

        <div className="input-hint">Scope</div>
        <select name="route-scope" defaultValue="0">
          <option value="0">Global</option>
          <option value="253">Link</option>
          <option value="254">Host</option>
        </select>

        <div className='actions'>
          <button onClick={() => {
            const srcFull = fuckItGetInputOptional('route-src');
            const dstFull = fuckItGetInputOptional('route-dst');
            const gateway = fuckItGetInputOptional('route-gateway');
            const dev = parseInt(fuckItGetInput('route-dev'));
            const table = parseInt(fuckItGetInputOptional('route-table') ?? '254');
            const metricRaw = fuckItGetInputOptional('route-metric');
            const metric = metricRaw === null ? null : parseInt(metricRaw);
            const scope = parseInt(fuckItGetInput('addr-scope'));

            let src = null;
            let dst = null;
            let family = 2; // V4
            if(srcFull !== null) {
              const [srcAddr, srcPlen] = srcFull.split('/')
              const isV4 = srcAddr.match(/\d+\.\d+\.\d+\.\d+/);
              if(!isV4) family = 10;
              src = [srcAddr, parseInt(srcPlen)];
            }
            if(dstFull !== null) {
              const [dstAddr, dstPlen] = dstFull.split('/')
              const isV4 = dstAddr.match(/\d+\.\d+\.\d+\.\d+/);
              if(!isV4) family = 10;
              dst = [dstAddr, parseInt(dstPlen)];
            }

            request('/route', 'POST', {
              family,
              table,
              scope,
              proto: 2,

              dst,
              src,
              gateway,
              dev,
              metric,
              // prefsrc: string | null;
            }).then(() => {
              updateAll();
              setShowAddRoute(false);
            });
          }}>Submit</button>
        </div>
      </div>
    </div>

    <div className={clsx("backdrop", { shown: showAddAddress })} onClick={exitDialog(setShowAddAddress)}>
      <div className="dialog">
        <div className="dialog-title">Add address</div>
        <div className="input-hint">Address</div>
        <input name="addr-address" />

        <div className="input-hint">Label</div>
        <input name="addr-label" placeholder='None' />

        <div className="input-hint">Device</div>
        <select name="addr-dev">
          {links.map((l, idx) => <option key={idx} value={l.index}>{l.ifname}(l.index)</option>)}
        </select>

        <div className="input-hint">Scope</div>
        <select name="addr-scope" defaultValue="0">
          <option value="0">Global</option>
          <option value="253">Link</option>
          <option value="254">Host</option>
        </select>

        <div className='actions'>
          <button onClick={() => {
            const addrFull = fuckItGetInput('addr-address');
            const label = fuckItGetInputOptional('addr-label');
            const index = parseInt(fuckItGetInput('addr-dev'));
            const scope = parseInt(fuckItGetInput('addr-scope'));

            const [address, plenStr] = addrFull.split('/')
            const isV4 = address.match(/\d+\.\d+\.\d+\.\d+/);
            const family = isV4 ? 2 : 10;
            const plen = parseInt(plenStr);

            request('/address', 'POST', {
              family, plen, scope, index,
              flags: 128,
              address,
              label,
              /*
              local: null | string,
              broadcast: null | string,
              */
            }).then(() => {
              updateAll();
              setShowAddAddress(false);
            });
          }}>Submit</button>
        </div>
      </div>
    </div>
  </>;
}

export default App;
