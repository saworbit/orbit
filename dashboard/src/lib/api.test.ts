import { describe, it, expect } from "vitest";
import { api } from "./api";

describe("API Configuration", () => {
  it("has correct base URL", () => {
    expect(api.defaults.baseURL).toBe("http://localhost:8080/api");
  });

  it("has withCredentials enabled for cookie support", () => {
    expect(api.defaults.withCredentials).toBe(true);
  });

  it("has correct default headers", () => {
    expect(api.defaults.headers["Content-Type"]).toBe("application/json");
  });

  it("has response interceptor configured", () => {
    expect(api.interceptors.response.handlers).toHaveLength(1);
  });
});
