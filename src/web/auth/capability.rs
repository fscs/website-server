use actix_utils::future::{Ready, ready};
use actix_web::body::EitherBody;
use actix_web::dev::{Service, ServiceRequest, ServiceResponse, Transform, forward_ready};
use actix_web::{HttpMessage, HttpResponse};
use futures_util::future::LocalBoxFuture;
use std::sync::Arc;

use crate::domain::Capability;

use super::User;

macro_rules! capability_middleware {
    ($transform:ident, $middleware:ident, $cap:expr) => {
        pub struct $transform;

        impl<S, B> Transform<S, ServiceRequest> for $transform
        where
            S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = actix_web::Error>
                + 'static,
            S::Future: 'static,
            B: 'static,
        {
            type Response = ServiceResponse<EitherBody<B>>;
            type Error = actix_web::Error;
            type Transform = $middleware<S>;
            type InitError = ();
            type Future = Ready<Result<Self::Transform, Self::InitError>>;

            fn new_transform(&self, service: S) -> Self::Future {
                ready(Ok($middleware {
                    service: Arc::new(service),
                }))
            }
        }

        pub struct $middleware<S> {
            service: Arc<S>,
        }

        impl<S, B> Service<ServiceRequest> for $middleware<S>
        where
            S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = actix_web::Error>
                + 'static,
            S::Future: 'static,
            B: 'static,
        {
            type Response = ServiceResponse<EitherBody<B>>;
            type Error = actix_web::Error;
            type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

            forward_ready!(service);

            fn call(&self, req: ServiceRequest) -> Self::Future {
                let service = self.service.clone();
                Box::pin(async move {
                    if req
                        .extensions()
                        .get::<User>()
                        .is_some_and(|user| !user.has_capability($cap))
                    {
                        return Ok(req.into_response(
                            HttpResponse::Unauthorized().finish().map_into_right_body(),
                        ));
                    }

                    Ok(service.call(req).await?.map_into_left_body())
                })
            }
        }
    };
}

capability_middleware!(
    RequireManageSitzungen,
    RequireManageSitzungenMiddleware,
    Capability::ManageSitzungen
);

capability_middleware!(
    RequireManageAnträge,
    RequireManageAnträgeMiddleware,
    Capability::ManageAnträge
);

capability_middleware!(
    RequireManagePersons,
    RequireManagePersonsMiddleware,
    Capability::ManagePersons
);

capability_middleware!(
    RequireManageDoor,
    RequireManageDoorMiddleware,
    Capability::ManageDoor
);

capability_middleware!(
    RequireCreateAntrag,
    RequireCreateAntragMiddleware,
    Capability::CreateAntrag
);

capability_middleware!(
    RequireViewHidden,
    RequireViewHiddenMiddleware,
    Capability::ViewHidden
);

capability_middleware!(
    RequireViewProtected,
    RequireViewProtectedMiddleware,
    Capability::ViewProtected
);
