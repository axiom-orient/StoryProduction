export class CdpClient {
  #socket;
  #nextId = 1;
  #pending = new Map();
  #eventWaiters = new Map();
  #eventListeners = new Map();

  constructor(socket) {
    this.#socket = socket;
    socket.addEventListener('message', (event) => this.#onMessage(event.data));
    socket.addEventListener('close', () => this.#rejectAll(new Error('CDP websocket closed')));
    socket.addEventListener('error', () => this.#rejectAll(new Error('CDP websocket error')));
  }

  static async connect(url, timeoutMs = 10_000) {
    const socket = new WebSocket(url);
    await new Promise((resolve, reject) => {
      const timer = setTimeout(() => reject(new Error(`Timed out connecting to CDP: ${url}`)), timeoutMs);
      socket.addEventListener('open', () => { clearTimeout(timer); resolve(); }, { once: true });
      socket.addEventListener('error', () => { clearTimeout(timer); reject(new Error(`Failed to connect to CDP: ${url}`)); }, { once: true });
    });
    return new CdpClient(socket);
  }

  async send(method, params = {}, timeoutMs = 15_000) {
    const id = this.#nextId++;
    const payload = JSON.stringify({ id, method, params });
    return new Promise((resolve, reject) => {
      const timer = setTimeout(() => {
        this.#pending.delete(id);
        reject(new Error(`CDP command timed out: ${method}`));
      }, timeoutMs);
      this.#pending.set(id, { resolve, reject, timer, method });
      this.#socket.send(payload);
    });
  }

  on(method, listener) {
    const listeners = this.#eventListeners.get(method) ?? new Set();
    listeners.add(listener);
    this.#eventListeners.set(method, listeners);
    return () => listeners.delete(listener);
  }

  waitFor(method, timeoutMs = 15_000) {
    return new Promise((resolve, reject) => {
      const waiters = this.#eventWaiters.get(method) ?? [];
      const entry = { resolve, reject, timer: undefined };
      entry.timer = setTimeout(() => {
        const current = this.#eventWaiters.get(method) ?? [];
        this.#eventWaiters.set(method, current.filter((item) => item !== entry));
        reject(new Error(`CDP event timed out: ${method}`));
      }, timeoutMs);
      waiters.push(entry);
      this.#eventWaiters.set(method, waiters);
    });
  }

  close() {
    this.#socket.close();
  }

  #onMessage(raw) {
    const message = JSON.parse(String(raw));
    if (message.id !== undefined) {
      const pending = this.#pending.get(message.id);
      if (!pending) return;
      clearTimeout(pending.timer);
      this.#pending.delete(message.id);
      if (message.error) pending.reject(new Error(`${pending.method}: ${message.error.message}`));
      else pending.resolve(message.result ?? {});
      return;
    }
    if (!message.method) return;
    const waiters = this.#eventWaiters.get(message.method) ?? [];
    this.#eventWaiters.set(message.method, []);
    for (const waiter of waiters) {
      clearTimeout(waiter.timer);
      waiter.resolve(message.params ?? {});
    }
    for (const listener of this.#eventListeners.get(message.method) ?? []) {
      try { listener(message.params ?? {}); } catch { /* listener evidence only */ }
    }
  }

  #rejectAll(error) {
    for (const pending of this.#pending.values()) {
      clearTimeout(pending.timer);
      pending.reject(error);
    }
    this.#pending.clear();
    for (const waiters of this.#eventWaiters.values()) {
      for (const waiter of waiters) {
        clearTimeout(waiter.timer);
        waiter.reject(error);
      }
    }
    this.#eventWaiters.clear();
  }
}
