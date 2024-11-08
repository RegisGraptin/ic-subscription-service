export const idlFactory = ({ IDL }) => {
  const Result = IDL.Variant({ 'Ok' : IDL.Text, 'Err' : IDL.Text });
  return IDL.Service({
    'transfer_usdc_periodically' : IDL.Func([], [Result], []),
  });
};
export const init = ({ IDL }) => { return []; };
