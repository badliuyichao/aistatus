"""Background worker for fetching API data off the GUI thread.

The three API fetchers (DeepSeek / GLM / MiniMax) each do a blocking
``urllib`` request with a ~10s timeout. Running them on the GUI thread
makes the whole window freeze for seconds on every refresh — startup,
the 60s timer, F5, and any balance.json edit. This worker moves all of
that network I/O onto a dedicated thread and runs the three requests
**in parallel**, so the wall-clock wait drops from ~3×RTT to ~1×RTT and
the GUI stays responsive the whole time.

Usage: ``FetchWorker`` is a ``QObject`` meant to be moved onto a
``QThread``. The GUI thread asks for a refresh by emitting
``fetch_requested`` (connected to ``request_fetch``); results come back
on ``fetched`` as three dicts/None, to be merged + rendered on the GUI
thread.
"""

from concurrent.futures import ThreadPoolExecutor

from PySide6.QtCore import QObject, Signal, Slot


class FetchWorker(QObject):
    """Fetch all three API sources in parallel on a worker thread.

    Lives on a dedicated ``QThread`` (the owner moves it there). The GUI
    triggers a fetch via the connected ``request_fetch`` slot; once all
    three requests resolve, ``fetched`` is emitted carrying the results
    back to the GUI thread.
    """

    # Emitted with (deepseek_data, glm_data, minimax_data). Each element is
    # the fetcher's result dict, or None when there is no key / the call
    # failed / the fetcher is absent.
    fetched = Signal(object, object, object)

    def __init__(self, deepseek_fetcher=None, glm_fetcher=None,
                 minimax_fetcher=None, parent=None):
        super().__init__(parent)
        self._deepseek = deepseek_fetcher
        self._glm = glm_fetcher
        self._minimax = minimax_fetcher
        self._busy = False  # guards against re-entrant requests piling up

    @Slot()
    def request_fetch(self):
        """Run all three fetchers in parallel, then emit the results.

        Invoked on the worker thread. Each ``fetch()`` may block up to
        ~10s; with three running concurrently the overall wait is ~1×
        instead of 3×. If a previous fetch is still in flight, the later
        request is dropped — the in-flight one will emit fresher data
        anyway.
        """
        if self._busy:
            return
        self._busy = True
        try:
            with ThreadPoolExecutor(max_workers=3) as pool:
                fut_ds = pool.submit(self._safe_fetch, self._deepseek)
                fut_glm = pool.submit(self._safe_fetch, self._glm)
                fut_mm = pool.submit(self._safe_fetch, self._minimax)
                ds_data = fut_ds.result()
                glm_data = fut_glm.result()
                mm_data = fut_mm.result()
            self.fetched.emit(ds_data, glm_data, mm_data)
        finally:
            self._busy = False

    @staticmethod
    def _safe_fetch(fetcher):
        """Call ``fetcher.fetch()`` returning None on any failure.

        The worker thread must never let an exception escape — Qt would
        swallow it silently and the ``_busy`` guard would never clear.
        """
        if fetcher is None or not getattr(fetcher, "has_key", False):
            return None
        try:
            return fetcher.fetch()
        except Exception:
            return None
