import { useEffect, useState } from "react";

export type CachedDataState<T> = {
  data: T | null;
  error: unknown;
  isLoading: boolean;
};

export function createCachedData<T>(load: () => Promise<T>) {
  let data: T | null = null;
  let error: unknown = null;
  let promise: Promise<T> | null = null;

  async function getData() {
    if (data !== null) {
      return data;
    }

    if (promise === null) {
      promise = load()
        .then((value) => {
          data = value;
          error = null;
          return value;
        })
        .catch((err) => {
          error = err;
          promise = null;
          throw err;
        });
    }

    return promise;
  }

  function useData(): CachedDataState<T> {
    const [state, setState] = useState<CachedDataState<T>>({
      data,
      error,
      isLoading: data === null && error === null,
    });

    useEffect(() => {
      let isMounted = true;

      getData()
        .then((value) => {
          if (isMounted) {
            setState({ data: value, error: null, isLoading: false });
          }
        })
        .catch((err) => {
          if (isMounted) {
            setState({ data: null, error: err, isLoading: false });
          }
        });

      return () => {
        isMounted = false;
      };
    }, []);

    return state;
  }

  function clearData() {
    data = null;
    error = null;
    promise = null;
  }

  return {
    clearData,
    getData,
    useData,
  };
}
