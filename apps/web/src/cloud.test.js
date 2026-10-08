import assert from 'node:assert/strict';
import test from 'node:test';
import { createCloud } from './cloud.js';

function fixture({ rows = [], users = ['alice'], error = null } = {}) {
  const calls = [];
  let sessionCalls = 0;
  const browser = {
    location: { origin: 'https://blobatar.example', pathname: '/' },
    sessionStorage: {
      data: new Map(),
      getItem(key) { return this.data.get(key) ?? null; },
      setItem(key, value) { this.data.set(key, value); },
      removeItem(key) { this.data.delete(key); },
    },
  };
  const query = {
    then(resolve, reject) { return Promise.resolve({ data: rows, error }).then(resolve, reject); },
  };
  for (const method of ['select', 'insert', 'update', 'delete', 'eq', 'order', 'range']) {
    query[method] = (...args) => { calls.push([method, ...args]); return query; };
  }
  const client = {
    from(table) { calls.push(['from', table]); return query; },
    auth: {
      async getSession() {
        const id = users[Math.min(sessionCalls++, users.length - 1)];
        return { data: id ? { user: { id, name: id } } : null };
      },
      signIn: { async social(options) { calls.push(['social', options]); return { data: {} }; } },
      async signOut() { return { data: {} }; },
    },
  };
  return { cloud: createCloud(client, browser), browser, calls };
}

test('GitHub redirect preserves the draft without requesting repo permissions', async () => {
  const { cloud, calls } = fixture();
  await cloud.login('{"name":"あばたー"}');
  assert.deepEqual(calls[0], ['social', { provider: 'github', callbackURL: 'https://blobatar.example/' }]);
  assert.equal(cloud.takeDraft(), '{"name":"あばたー"}');
  assert.equal(cloud.takeDraft(), null);
});

test('list paginates without exposing the extra row', async () => {
  const rows = Array.from({ length: 13 }, (_, id) => ({ id }));
  const { cloud, calls } = fixture({ rows });
  const result = JSON.parse(await cloud.list(12));
  assert.equal(result.rows.length, 12);
  assert.equal(result.more, true);
  assert.deepEqual(calls.find(call => call[0] === 'range'), ['range', 12, 24]);
});

test('updates require both ID and revision; copies never send owner or ID', async () => {
  const { cloud, calls } = fixture({ rows: [{ id: 'work', updated_at: 'new' }] });
  await cloud.save('{"name":"changed"}', 'work', 'old');
  assert.ok(calls.some(call => JSON.stringify(call) === JSON.stringify(['eq', 'id', 'work'])));
  assert.ok(calls.some(call => JSON.stringify(call) === JSON.stringify(['eq', 'updated_at', 'old'])));
  calls.length = 0;
  await cloud.save('{"name":"copy"}', '', '');
  assert.deepEqual(calls.find(call => call[0] === 'insert'), ['insert', { settings: { name: 'copy' } }]);
  assert.equal(calls.some(call => call[0] === 'eq'), false);
});

test('a missing revision match is an error, not a successful save/delete', async () => {
  const { cloud } = fixture();
  await assert.rejects(cloud.save('{}', 'work', 'old'), /changed or deleted/);
  await assert.rejects(cloud.remove('work', 'old'), /changed or deleted/);
});

test('sign-out or account change during a request discards its result', async () => {
  for (const users of [['alice', 'bob'], ['alice', null]]) {
    const { cloud } = fixture({ users });
    await assert.rejects(cloud.list(0), /Account changed/);
  }
  const { cloud, calls } = fixture({ users: [null] });
  await assert.rejects(cloud.list(0), /sign in again/);
  assert.deepEqual(calls, []);
});
