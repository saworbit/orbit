import { describe, it, expect, beforeAll, afterAll } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { FileBrowser } from "./FileBrowser";

/**
 * Integration tests for FileBrowser component
 *
 * These tests verify the full interaction flow between the FileBrowser
 * component and the backend API.
 *
 * NOTE: These tests require the backend to be running on localhost:3000
 * Run with: npm test -- FileBrowser.integration.test.tsx
 */

describe("FileBrowser Integration Tests", () => {
  let queryClient: QueryClient;
  let mockOnSelect: (path: string) => void;
  let selectedPaths: string[] = [];

  beforeAll(() => {
    queryClient = new QueryClient({
      defaultOptions: {
        queries: {
          retry: false,
          gcTime: 0,
        },
      },
    });

    mockOnSelect = (path: string) => {
      selectedPaths.push(path);
    };
  });

  afterAll(() => {
    queryClient.clear();
  });

  it.skip("should load and display files from the API", async () => {
    render(
      <QueryClientProvider client={queryClient}>
        <FileBrowser onSelect={mockOnSelect} />
      </QueryClientProvider>
    );

    // Wait for loading to complete
    await waitFor(
      () => {
        expect(screen.queryByText(/Loading.../i)).not.toBeInTheDocument();
      },
      { timeout: 5000 }
    );

    // Verify files are displayed
    const items = screen.getByText(/items/i);
    expect(items).toBeInTheDocument();
  });

  it.skip("should navigate into a folder when clicked", async () => {
    const user = userEvent.setup();

    render(
      <QueryClientProvider client={queryClient}>
        <FileBrowser onSelect={mockOnSelect} />
      </QueryClientProvider>
    );

    // Wait for initial load
    await waitFor(
      () => {
        expect(screen.queryByText(/Loading.../i)).not.toBeInTheDocument();
      },
      { timeout: 5000 }
    );

    // Find and click a folder (assuming there's at least one)
    const folders = screen.getAllByRole("button");
    if (folders.length > 0) {
      await user.click(folders[0]);

      // Wait for navigation to complete
      await waitFor(
        () => {
          expect(screen.queryByText(/Loading.../i)).not.toBeInTheDocument();
        },
        { timeout: 5000 }
      );
    }
  });

  it.skip("should select a file when clicked", async () => {
    const user = userEvent.setup();
    selectedPaths = []; // Reset

    render(
      <QueryClientProvider client={queryClient}>
        <FileBrowser onSelect={mockOnSelect} />
      </QueryClientProvider>
    );

    await waitFor(
      () => {
        expect(screen.queryByText(/Loading.../i)).not.toBeInTheDocument();
      },
      { timeout: 5000 }
    );

    // Find a file (not a folder) and click it
    const fileElements = screen.getAllByRole("generic");
    if (fileElements.length > 0) {
      await user.click(fileElements[0]);

      // Verify onSelect was called
      expect(selectedPaths.length).toBeGreaterThan(0);
    }
  });

  it.skip("should use 'Select Current Folder' button", async () => {
    const user = userEvent.setup();
    selectedPaths = []; // Reset

    render(
      <QueryClientProvider client={queryClient}>
        <FileBrowser onSelect={mockOnSelect} />
      </QueryClientProvider>
    );

    await waitFor(
      () => {
        expect(screen.queryByText(/Loading.../i)).not.toBeInTheDocument();
      },
      { timeout: 5000 }
    );

    // Click "Select Current Folder" button
    const selectButton = screen.getByText(/Select Current Folder/i);
    await user.click(selectButton);

    // Verify current path was selected
    expect(selectedPaths.length).toBeGreaterThan(0);
  });

  it.skip("should navigate up when up arrow is clicked", async () => {
    const user = userEvent.setup();

    render(
      <QueryClientProvider client={queryClient}>
        <FileBrowser onSelect={mockOnSelect} />
      </QueryClientProvider>
    );

    await waitFor(
      () => {
        expect(screen.queryByText(/Loading.../i)).not.toBeInTheDocument();
      },
      { timeout: 5000 }
    );

    // Navigate into a folder first
    const folders = screen.getAllByRole("button");
    if (folders.length > 1) {
      await user.click(folders[1]); // Click second button (first might be up arrow)

      await waitFor(
        () => {
          expect(screen.queryByText(/Loading.../i)).not.toBeInTheDocument();
        },
        { timeout: 5000 }
      );

      // Now click the up arrow
      const upButton = screen.getAllByRole("button")[0];
      await user.click(upButton);

      await waitFor(
        () => {
          expect(screen.queryByText(/Loading.../i)).not.toBeInTheDocument();
        },
        { timeout: 5000 }
      );
    }
  });

  it.skip("should show error message when API is unreachable", async () => {
    // This test would require mocking the API to return an error
    // or temporarily stopping the backend

    render(
      <QueryClientProvider client={queryClient}>
        <FileBrowser onSelect={mockOnSelect} />
      </QueryClientProvider>
    );

    // Wait for error message
    await waitFor(
      () => {
        const errorMessage = screen.queryByText(/Failed to list files/i);
        expect(errorMessage).toBeInTheDocument();
      },
      { timeout: 5000 }
    );
  });
});

/**
 * TODO: Add more integration tests
 *
 * - Test keyboard navigation (arrow keys)
 * - Test search/filter functionality (if added)
 * - Test drag and drop (if added)
 * - Test context menu (if added)
 * - Test multi-select (if added)
 * - Test sorting options (if added)
 * - Test view modes (list/grid) (if added)
 */
