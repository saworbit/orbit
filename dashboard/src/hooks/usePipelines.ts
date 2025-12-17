import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { api } from "../lib/api";
import { Node, Edge } from "@xyflow/react";

// Pipeline types matching backend
export interface PipelineInfo {
  id: string;
  name: string;
  description: string;
  status: string;
  created_at: number;
  updated_at: number;
}

export interface PipelineDetail {
  id: string;
  name: string;
  description: string;
  status: string;
  nodes: Node[];
  edges: Edge[];
  created_at: number;
  updated_at: number;
}

export interface CreatePipelineRequest {
  name: string;
  description?: string;
}

/**
 * Hook to fetch all pipelines
 */
export function usePipelines() {
  return useQuery({
    queryKey: ["pipelines"],
    queryFn: async () => {
      const res = await api.post<PipelineInfo[]>("/list_pipelines");
      return res.data;
    },
    refetchInterval: 5000, // Refresh every 5 seconds
  });
}

/**
 * Hook to fetch a single pipeline with full node/edge details
 */
export function usePipeline(id: string) {
  return useQuery({
    queryKey: ["pipeline", id],
    queryFn: async () => {
      const res = await api.post<PipelineDetail>("/get_pipeline", {
        pipeline_id: id,
      });
      return res.data;
    },
    enabled: !!id,
    refetchInterval: 2000, // Fast polling for visual editor
  });
}

/**
 * Hook to create a new pipeline
 */
export function useCreatePipeline() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: async (request: CreatePipelineRequest) => {
      const res = await api.post<string>("/create_pipeline", {
        name: request.name,
        description: request.description || "",
      });
      return res.data; // Returns pipeline ID
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["pipelines"] });
    },
  });
}

/**
 * Hook to save pipeline (bulk snapshot update)
 * This replaces the chatty add_node/remove_edge pattern
 */
export function useSavePipeline() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: async ({
      id,
      nodes,
      edges,
    }: {
      id: string;
      nodes: Node[];
      edges: Edge[];
    }) => {
      const res = await api.post<string>("/sync_pipeline", {
        pipeline_id: id,
        nodes_json: JSON.stringify(nodes),
        edges_json: JSON.stringify(edges),
      });
      return res.data;
    },
    onSuccess: (_, variables) => {
      queryClient.invalidateQueries({ queryKey: ["pipeline", variables.id] });
      queryClient.invalidateQueries({ queryKey: ["pipelines"] });
    },
  });
}

/**
 * Hook to delete a pipeline
 */
export function useDeletePipeline() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: async (pipelineId: string) => {
      const res = await api.post<string>("/delete_pipeline", {
        pipeline_id: pipelineId,
      });
      return res.data;
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["pipelines"] });
    },
  });
}
