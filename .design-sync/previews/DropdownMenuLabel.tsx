import { Button, DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuLabel, DropdownMenuSeparator, DropdownMenuTrigger, EllipsisVertical, FolderInput, Pencil, Trash2 } from 'kosmotheka';

export const BookMenu = () => (
  <div className="flex justify-end p-4">
    <DropdownMenu open modal={false}>
      <DropdownMenuTrigger asChild>
        <Button variant="ghost" size="icon" aria-label="More options">
          <EllipsisVertical />
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end" className="min-w-56" onCloseAutoFocus={(e) => e.preventDefault()}>
        <DropdownMenuLabel>Dune</DropdownMenuLabel>
        <DropdownMenuItem><Pencil /> Edit</DropdownMenuItem>
        <DropdownMenuItem><FolderInput /> Move to another library</DropdownMenuItem>
        <DropdownMenuSeparator />
        <DropdownMenuItem variant="destructive"><Trash2 /> Delete</DropdownMenuItem>
      </DropdownMenuContent>
    </DropdownMenu>
  </div>
);
