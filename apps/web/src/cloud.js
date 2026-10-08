import { createClient } from '@neondatabase/neon-js';

export function createCloud(client, browser) {
const draftKey = 'blobatar:login-draft';
const pageSize = 12;

function requireClient() {
  if (!client) throw new Error('Cloud storage is not configured');
  return client;
}

function checked(result) {
  if (result.error) throw new Error(result.error.message || 'Cloud request failed');
  return result.data;
}

async function session() {
  const data = checked(await requireClient().auth.getSession());
  return data?.user ? { id: data.user.id, name: data.user.name || 'GitHub user' } : null;
}

async function withUser(operation) {
  const before = await session();
  if (!before) throw new Error('Please sign in again');
  const result = await operation();
  const after = await session();
  if (!after || after.id !== before.id) throw new Error('Account changed; reload My Wall');
  return JSON.stringify({ user: after, ...result });
}

return {
  configured: () => Boolean(client),
  session: async () => JSON.stringify(await session()),
  takeDraft: () => {
    const json = browser.sessionStorage.getItem(draftKey);
    browser.sessionStorage.removeItem(draftKey);
    return json;
  },
  login: async (settings) => {
    browser.sessionStorage.setItem(draftKey, settings);
    checked(await requireClient().auth.signIn.social({
      provider: 'github',
      callbackURL: browser.location.origin + browser.location.pathname,
    }));
  },
  logout: async () => { checked(await requireClient().auth.signOut()); },
  list: async (offset) => withUser(async () => {
    const rows = checked(await client.from('blobatar_avatars')
      .select('id,settings,updated_at')
      .order('updated_at', { ascending: false })
      .order('id')
      .range(offset, offset + pageSize));
    return { rows: rows.slice(0, pageSize), more: rows.length > pageSize };
  }),
  save: async (settings, id, revision) => withUser(async () => {
    const payload = { settings: JSON.parse(settings) };
    const query = id
      ? client.from('blobatar_avatars').update(payload).eq('id', id).eq('updated_at', revision)
      : client.from('blobatar_avatars').insert(payload);
    const rows = checked(await query.select('id,settings,updated_at'));
    if (rows.length !== 1) throw new Error('作品が別の端末で変更・削除されました。一覧を更新してください / Work changed or deleted; refresh My Wall');
    return { row: rows[0] };
  }),
  remove: async (id, revision) => withUser(async () => {
    const rows = checked(await client.from('blobatar_avatars').delete()
      .eq('id', id).eq('updated_at', revision).select('id'));
    if (rows.length !== 1) throw new Error('Work changed or deleted; refresh My Wall');
    return {};
  }),
};
}

if (typeof window !== 'undefined') {
  const endpoint = NEON_PUBLIC_DATABASE_URL;
  window.blobatarCloud = createCloud(endpoint ? createClient(endpoint) : null, window);
}
