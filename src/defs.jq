def highlight($message): {
  kind: "_treeq_highlight",
  start_byte: .start_byte,
  end_byte: .end_byte,
  message: $message,
};

def replace($entries): {
  kind: "_treeq_replace",
  start_byte: .start_byte,
  end_byte: .end_byte,
  entries: $entries
};
