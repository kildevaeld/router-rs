//
export default (router) => {
  //
  console.log("init from js");
  router.get("/", (req) => {
    console.log("Helle, World!");
    return new Response("Hello from js");
  });
};
