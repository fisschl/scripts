export interface FileItem {
  path: string
  status: 'pending' | 'processing'
}

export interface FileInfo {
  path: string
  is_dir: boolean
  size: number
}
