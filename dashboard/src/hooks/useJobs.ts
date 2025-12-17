import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { api } from "../lib/api";

export interface JobInfo {
  id: number;
  source: string;
  destination: string;
  status: string;
  progress: number;
  total_chunks: number;
  completed_chunks: number;
  failed_chunks: number;
  created_at: number;
  updated_at: number;
}

export interface CreateJobRequest {
  source: string;
  destination: string;
  compress?: boolean;
  verify?: boolean;
  parallel_workers?: number;
}

export function useJobs() {
  return useQuery({
    queryKey: ["jobs"],
    queryFn: async () => {
      const res = await api.post<JobInfo[]>("/list_jobs");
      return res.data;
    },
    refetchInterval: 2000, // Refresh every 2 seconds
  });
}

export function useCreateJob() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: async (request: CreateJobRequest) => {
      const res = await api.post<number>("/create_job", {
        source: request.source,
        destination: request.destination,
        compress: request.compress ?? false,
        verify: request.verify ?? true,
        parallel_workers: request.parallel_workers ?? 4,
      });
      return res.data;
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["jobs"] });
    },
  });
}

export function useRunJob() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: async (jobId: number) => {
      const res = await api.post("/run_job", { job_id: jobId });
      return res.data;
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["jobs"] });
    },
  });
}

export function useCancelJob() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: async (jobId: number) => {
      const res = await api.post("/cancel_job", { job_id: jobId });
      return res.data;
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["jobs"] });
    },
  });
}

export function useDeleteJob() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: async (jobId: number) => {
      const res = await api.post("/delete_job", { job_id: jobId });
      return res.data;
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["jobs"] });
    },
  });
}

export function useJobDetail(jobId: number) {
  return useQuery({
    queryKey: ["job", jobId],
    queryFn: async () => {
      const res = await api.post<JobInfo>("/get_job", { job_id: jobId });
      return res.data;
    },
    refetchInterval: 1000, // Poll every second for live updates
    enabled: jobId > 0,
  });
}
